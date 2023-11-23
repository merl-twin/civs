use serde::{
    Serialize,Deserialize,
    ser::{Serializer,SerializeStruct},
    de::DeserializeOwned,
};
use byteorder::{LittleEndian,ReadBytesExt,WriteBytesExt};
use std::io::{Read,Write};
use crate::{
    Flags,Filled,Binary,
    civs::{Slot,TOMBS_LIMIT,AUTO_SHRINK_LIMIT},
};

pub enum RemovedItem<'t,V> {
    Ref(&'t mut V),
    Owned(V),
}
impl<'t,V> RemovedItem<'t,V> {
    pub fn swap(self, mut tmp: V) -> V {
        match self {
            RemovedItem::Ref(r) => {
                std::mem::swap(&mut tmp, r);
                tmp
            },
            RemovedItem::Owned(v) => v,
        }
    }
}
impl<'t,V> AsRef<V> for RemovedItem<'t,V> {
    fn as_ref(&self) -> &V {
        match self {
            RemovedItem::Ref(r) => r,
            RemovedItem::Owned(v) => v,
        }
    }
}
impl<'t,V> AsMut<V> for RemovedItem<'t,V> {
    fn as_mut(&mut self) -> &mut V {
        match self {
            RemovedItem::Ref(r) => r,
            RemovedItem::Owned(v) => v,
        }
    }
}
impl<'t,V: Copy> RemovedItem<'t,V> {
    pub fn copied(self) -> V {
        match self {
            RemovedItem::Ref(r) => *r,
            RemovedItem::Owned(v) => v,
        }
    }
}
impl<'t,V: Clone> RemovedItem<'t,V> {
    pub fn cloned(self) -> V {
        match self {
            RemovedItem::Ref(r) => r.clone(),
            RemovedItem::Owned(v) => v,
        }
    }
}

#[derive(Deserialize)]
struct SerdeMapMultiSlot<K,V> {
    capacity: usize,
    key_size: usize,
    value_size: usize,
    flags: Vec<u64>,
    keys: Vec<K>,
    values: Vec<V>,
}
impl<K,V> std::convert::TryFrom<SerdeMapMultiSlot<K,V>> for MapMultiSlot<K,V> {
    type Error = String;
    fn try_from(mut slot: SerdeMapMultiSlot<K,V>) -> Result<MapMultiSlot<K,V>,String> {
        if slot.key_size != std::mem::size_of::<K>() { return Err(format!("Unvalid key size {}, must be {}",std::mem::size_of::<K>(),slot.key_size)); }
        if slot.value_size != std::mem::size_of::<V>() { return Err(format!("Unvalid value size {}, must be {}",std::mem::size_of::<V>(),slot.value_size)); }
        if (slot.keys.len() > 0) && (slot.keys.len() < slot.capacity) {
            slot.keys.reserve(slot.capacity - slot.keys.len());
        }
        if (slot.values.len() > 0) && (slot.values.len() < slot.capacity) {
            slot.values.reserve(slot.capacity - slot.values.len());
        }
        Ok(MapMultiSlot {
            capacity: slot.capacity,
            flags: Flags(slot.flags),
            keys: slot.keys,
            values: slot.values,
        })
    }
}

impl<K, V> Serialize for MapMultiSlot<K,V>
where
    K: Serialize,
    V: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SerdeMapMultiSlot", 6)?;
        state.serialize_field("capacity", &self.capacity)?;
        state.serialize_field("key_size", &std::mem::size_of::<K>())?;
        state.serialize_field("value_size", &std::mem::size_of::<V>())?;
        state.serialize_field("flags", &self.flags)?;
        state.serialize_field("keys", &self.keys)?;
        state.serialize_field("values", &self.values)?;
        state.end()
    }
}

#[derive(Debug,Clone,Deserialize)]
#[serde(try_from = "SerdeMapMultiSlot<K,V>")]
pub(crate) struct MapMultiSlot<K,V> {
    capacity: usize,
    flags: Flags,
    keys: Vec<K>,
    values: Vec<V>,
}
impl<K,V> MapMultiSlot<K,V> {
    fn heap_mem(&self) -> usize {
        self.flags.heap_mem() + self.keys.capacity() * std::mem::size_of::<K>() + self.values.capacity() * std::mem::size_of::<V>()
    }
}
impl<K: Ord, V> MapMultiSlot<K,V> {
    pub(crate) fn new(data: Vec<(K,V)>) -> MapMultiSlot<K,V> {
        let len = data.len();
        let mut keys = Vec::with_capacity(len);
        let mut values = Vec::with_capacity(len);
        for (k,v) in data {
            keys.push(k);
            values.push(v);
        }
        MapMultiSlot {
            capacity: len,
            flags: Flags::ones(len),
            keys: keys,
            values: values,
        }
    }
    fn new_empty(sz: usize, slot_sz: usize) -> MapMultiSlot<K,V> {
        let cap = slot_sz * (0x1 << (sz-1));
        MapMultiSlot {
            capacity: cap,
            flags: Flags::nulls(cap),
            keys: Vec::with_capacity(cap),
            values: Vec::with_capacity(cap),
        }
    }
    fn empty(&self) -> bool {
        self.keys.len() == 0
    }
    fn check_len(&self) -> usize {
        self.flags.0.iter().fold(0,|acc,x| acc + x.count_ones() as usize)
    }
    fn contains(&self, k: &K) -> Option<usize> {
        if (self.keys.len() == 0)||(*k < self.keys[0])||(*k > self.keys[self.keys.len()-1]) { return None; }
        match self.keys.binary_search(k) {
            Ok(idx) => match self.flags.get(idx) {
                true => Some(idx),
                false => None,
            },
            Err(_) => None,
        }
    }            
    fn clear(&mut self) {
        self.flags.set_nulls();
        self.keys.clear();
        self.values.clear();
    }
    fn shrink_to_fit(&mut self) {
        self.keys.shrink_to_fit();
        self.values.shrink_to_fit();
    }
    fn reserve(&mut self, cnt: usize) {
        self.keys.reserve(cnt);
        self.values.reserve(cnt);
    }
    fn drain(&mut self) -> MapMultiSlotDrainIterator<K,V> {
        MapMultiSlotDrainIterator {
            iter: self.keys.drain(..).zip(self.values.drain(..)),
        }
    }
    fn filtered_drain(&mut self) -> MapMultiSlotFilterDrainIterator<K,V> {
        MapMultiSlotFilterDrainIterator {
            iter: self.keys.drain(..).zip(self.values.drain(..)).enumerate(),
            flags: &self.flags,
        }
    }
    fn filtered_iter(&self) -> MapMultiSlotFilterIterator<K,V> {
        MapMultiSlotFilterIterator {
            iter: self.keys.iter().zip(self.values.iter()).enumerate(),
            flags: &self.flags,
        }
    }
    fn fill_in<'t>(&mut self, iter: &mut std::iter::Zip<std::vec::Drain<'t,K>,std::vec::Drain<'t,V>>) -> bool { // is exhausted
        let mut cur = 0;
        while cur < self.capacity {
            match iter.next() {
                Some((k,v)) => {
                    self.keys.push(k);
                    self.values.push(v);
                },
                None => return true,
            }
            cur += 1;
        }
        return false;
    }
}

struct MapMultiSlotFilterIterator<'t,K,V> {
    iter: std::iter::Enumerate<std::iter::Zip<std::slice::Iter<'t,K>,std::slice::Iter<'t,V>>>,
    flags: &'t Flags,
}
impl<'t,K,V> Iterator for MapMultiSlotFilterIterator<'t,K,V> {
    type Item = (&'t K, &'t V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((n,(k,v))) if self.flags.get(n) => break Some((k,v)),
                Some(_) => continue,
                None => break None,
            }
        }
    }
}

struct MapMultiSlotFilterDrainIterator<'t,K,V> {
    iter: std::iter::Enumerate<std::iter::Zip<std::vec::Drain<'t,K>,std::vec::Drain<'t,V>>>,
    flags: &'t Flags,
}
impl<'t,K,V> Iterator for MapMultiSlotFilterDrainIterator<'t,K,V> {
    type Item = (K,V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((n,(k,v))) if self.flags.get(n) => break Some((k,v)),
                Some(_) => continue,
                None => break None,
            }
        }
    }
}

struct MapMultiSlotDrainIterator<'t,K,V> {
    iter: std::iter::Zip<std::vec::Drain<'t,K>,std::vec::Drain<'t,V>>,
}
impl<'t,K,V> Iterator for MapMultiSlotDrainIterator<'t,K,V> {
    type Item = (K,V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}


pub struct Iter<'t,K,V> {
    slot_iter: Option<std::slice::Iter<'t,(K,V)>>,
    cur_data_iter: Option<MapMultiSlotFilterIterator<'t,K,V>>,
    data_iter: Vec<MapMultiSlotFilterIterator<'t,K,V>>,
}
impl<'t,K,V> Iterator for Iter<'t,K,V> {
    type Item = (&'t K, &'t V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.slot_iter {
            match iter.next() {
                Some((k,v)) => return Some((&k, &v)),
                None => { self.slot_iter = None; },
            }
        }
        while let Some(iter) = &mut self.cur_data_iter {
            match iter.next() {
                Some((k,v)) => return Some((k, v)),
                None => { self.cur_data_iter = self.data_iter.pop(); },
            }
        }
        None
    }
}


const CURRENT_CIVS_MAP_VERSION: (u32,u32) = (0,1);

#[derive(Debug)]
pub enum CivMapIoError {
    WriteHeader,
    WriteSlot(bincode::Error),
    WriteData(bincode::Error),
    ReadHeader,
    ReadSlot(bincode::Error),
    ReadData(bincode::Error),
    InvalidHeader,
    InvalidVersion(u32,u32),
}

impl<K: Ord + Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> Binary for CivMap<K,V> {
    type IoError = CivMapIoError;
    fn memory(&self) -> usize {
        let mut data_mem = self.data.capacity() * std::mem::size_of::<MapMultiSlot<K,V>>();
        for ms in &self.data {
            data_mem += ms.heap_mem();
        }
        std::mem::size_of::<CivMap<K,V>>() + self.slot.heap_mem() + data_mem
    }
    fn into_writer<W: Write>(&self, mut wrt: W) -> Result<(),Self::IoError> {
        let version = CURRENT_CIVS_MAP_VERSION;
        write!(wrt,"CIVM").map_err(|_|CivMapIoError::WriteHeader)?;
        wrt.write_u32::<LittleEndian>(version.0).map_err(|_|CivMapIoError::WriteHeader)?;
        wrt.write_u32::<LittleEndian>(version.1).map_err(|_|CivMapIoError::WriteHeader)?;
        bincode::serialize_into(&mut wrt,&self.slot).map_err(CivMapIoError::WriteSlot)?;
        bincode::serialize_into(&mut wrt,&self.data).map_err(CivMapIoError::WriteData)
    }
    fn from_reader<R: Read>(mut rdr: R) -> Result<CivMap<K,V>,Self::IoError> {
        let mut buf = [0; 4];
        rdr.read_exact(&mut buf).map_err(|_|CivMapIoError::ReadHeader)?;
        if buf != "CIVM".as_bytes()[0..4] { return Err(CivMapIoError::InvalidHeader); }
        let maj = rdr.read_u32::<LittleEndian>().map_err(|_|CivMapIoError::ReadHeader)?;
        let min = rdr.read_u32::<LittleEndian>().map_err(|_|CivMapIoError::ReadHeader)?;
        if (maj != 0)||(min != 1) { return Err(CivMapIoError::InvalidVersion(maj,min)); }
        let slot: Slot<K,V> = bincode::deserialize_from(&mut rdr).map_err(CivMapIoError::ReadSlot)?;
        let data: Vec<MapMultiSlot<K,V>> = bincode::deserialize_from(&mut rdr).map_err(CivMapIoError::ReadData)?;
        let mut len = slot.len();
        let mut tombs = 0;
        for ms in &data {
            let ln = ms.check_len();
            len += ln;
            tombs += ms.capacity - ln;
        }
        Ok(CivMap {
            len: len,
            tombs: tombs,
            slot: slot,
            data: data,
            
            tmp_merge_keys: Vec::new(),
            tmp_merge_values: Vec::new(),
        })
    }
}



#[derive(Clone)]
pub struct CivMap<K,V> {
    len: usize,
    tombs: usize,
    slot: Slot<K,V>,
    data: Vec<MapMultiSlot<K,V>>,

    tmp_merge_keys: Vec<K>,
    tmp_merge_values: Vec<V>,
}
impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for CivMap<K,V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CivMap")
            .field("len", &self.len)
            .field("tombs", &self.tombs)
            .field("slot", &self.slot)
            .field("data", &self.data)
            .finish()
    }
}     
impl<K: Ord, V> CivMap<K,V> {
    pub fn new() -> CivMap<K,V> {
        CivMap {
            len: 0,
            tombs: 0,
            slot: Slot::new(),
            data: Vec::new(),

            tmp_merge_keys: Vec::new(),
            tmp_merge_values: Vec::new(),
        }
    }

    pub fn filtered_iter(&self) -> Iter<K,V> {
        let mut v = {
            let mut v = Vec::new();
            let n = self.data.len();
            for i in 0 .. n {
                v.push(self.data[n-i-1].filtered_iter());
            }
            v
        };
        Iter {
            slot_iter: Some(self.slot.iter()),
            cur_data_iter: v.pop(),
            data_iter: v,            
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.tombs = 0;
        self.slot.clear();
        self.data.clear();
        self.tmp_merge_keys.clear();
        self.tmp_merge_values.clear();
    }
    
    pub fn contains(&self, k: &K) -> bool {
        match self.slot.contains(k) {
            Some(_) => true,
            None => self.multy_contains(k).is_some(),
        }
    }    
    fn multy_contains(&self, k: &K) -> Option<(usize,usize)> {
        for (n,ms) in self.data.iter().enumerate() {
            if let Some(idx) = ms.contains(k) {
                return Some((n,idx));
            }
        }
        None
    }
    pub fn get(&self, k: &K) -> Option<&V> {
        match self.slot.get(k) {
            r @ Some(_) => r,
            None => match self.multy_contains(k) {
                Some((msi,idx)) => Some(&self.data[msi].values[idx]),
                None => None,
            }
        }
    }
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        match self.multy_contains(k) {
            Some((msi,idx)) => Some(&mut self.data[msi].values[idx]),
            None => self.slot.get_mut(k),
        }
    }
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if let Some((msi,idx)) = self.multy_contains(&k) {
            let mut tmp = v;
            std::mem::swap(&mut tmp, &mut self.data[msi].values[idx]);
            return Some(tmp);
        }
        let (r,filled) = self.slot.insert(k,v);
        if let Filled::Full = filled {
            if self.data.len() == 0 {
                self.data.push(self.slot.into_map_multislot());
            } else {
                let mut n = 0;
                while (n < self.data.len())&&(!self.data[n].empty()) { n += 1; }
                if n == self.data.len() {
                    self.data.push(MapMultiSlot::new_empty(n+1,self.slot.max_size()));
                }
                if let Err(s) = self.merge_into(n) {
                    panic!("Unreachable merge_into: {}",s);
                }
                if let Err(s) = self.check_tombs(n) {
                    panic!("Unreachable check_tombs: {}",s);
                }
                self.shrink_long();
            }
        }
        if r.is_none() {
            self.len += 1;
        }
        r
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn tombs(&self) -> usize {
        self.tombs
    }
    pub fn remove(&mut self, k: &K) -> Option<RemovedItem<V>> {
        let r = match self.multy_contains(&k) {
            Some((msi,idx)) => {
                self.tombs += 1;
                self.data[msi].flags.unset(idx);
                Some(RemovedItem::Ref(&mut self.data[msi].values[idx]))
            },
            None => match self.slot.remove(k) {
                Some(v) => Some(RemovedItem::Owned(v)),
                None => None,
            },
        };
        if r.is_some() {
            self.len -= 1;
        }
        r
    }
    pub fn shrink_to_fit(&mut self) {
        for ms in &mut self.data {
            ms.shrink_to_fit();
        }
    }
    fn shrink_long(&mut self) {
        for ms in &mut self.data {
            if (ms.capacity >= AUTO_SHRINK_LIMIT)&&(ms.empty()) {   
                ms.shrink_to_fit();
            }
        }
    }
    fn check_tombs(&mut self, n: usize) -> Result<(),&'static str> {
        if self.data[n].empty() { return Err("data[n] is empty"); }
        for i in 0 .. n {
            if !self.data[i].empty() { return Err("one of data[0..n] is not empty"); }
        }

        let sz =  self.slot.max_size();
        let local_tombs = self.data[n].capacity - self.data[n].keys.len();
        let local_part = (local_tombs as f64) / (self.data[n].capacity as f64);
        if (local_tombs > sz) && (local_part > TOMBS_LIMIT) {
            std::mem::swap(&mut self.data[n].keys, &mut self.tmp_merge_keys);
            std::mem::swap(&mut self.data[n].values, &mut self.tmp_merge_values);
            {
                let mut count = self.tmp_merge_keys.len();
                let mut iter = self.tmp_merge_keys.drain(..).zip(self.tmp_merge_values.drain(..));

                let mut msi = self.data[..n].iter_mut();
                while let Some(ms) = msi.next_back() {
                    let cap = ms.capacity;
                    if count >= cap {
                        for _ in 0 .. cap {
                            if let Some((k,v)) = iter.next() {
                                ms.keys.push(k);
                                ms.values.push(v);
                            }
                        }
                        if ms.keys.len() != cap {
                            return Err("data count < data.len()");
                        }
                        ms.flags.set_ones(cap);
                        count -= cap;
                        if count == 0 { break; }
                        continue;
                    }
                    if (cap - count) > sz { continue; }
                    // checked tombs = (cap - count) <= sz and local_tombs > sz
                    let d_tombs = local_tombs - (cap - count);
                    for _ in 0 .. count {
                        if let Some((k,v)) = iter.next() {
                            ms.keys.push(k);
                            ms.values.push(v);
                        }
                    }
                    if ms.keys.len() != count {
                        return Err("data count < data.len()");
                    }
                    ms.flags.set_ones(count);
                    if d_tombs > self.tombs {
                        return Err("local_tombs > self.tombs");
                    }
                    self.tombs -= d_tombs;
                    break;
                }
                if let Some(_) = iter.next() {
                    return Err("merged data greater then the sum of the parts");
                }
            }
            std::mem::swap(&mut self.data[n].keys, &mut self.tmp_merge_keys);
            std::mem::swap(&mut self.data[n].values, &mut self.tmp_merge_values);
            self.data[n].clear();
        }
        Ok(())
    }
    fn merge_into(&mut self, n: usize) -> Result<(),&'static str> {
        // merge sort for sorted inflating vectors
        
        if !self.data[n].empty() { return Err("data[n] is not empty"); }
        let mut cnt = self.slot.len();
        for i in 0 .. n {
            if self.data[i].empty() { return Err("one of data[0..n] is empty"); }
            cnt += self.data[i].keys.len();
        }
        self.data[n].reserve(cnt);

        std::mem::swap(&mut self.data[n].keys, &mut self.tmp_merge_keys);
        std::mem::swap(&mut self.data[n].values, &mut self.tmp_merge_values);
        {
            if n == 0 {
                for (k,v) in self.slot.sorted_drain() {
                    self.tmp_merge_keys.push(k);
                    self.tmp_merge_values.push(v);
                }
                self.slot.clear();
            } else {
                let mut slot = self.slot.into_map_multislot();
                self.slot.clear();
                for i in 0 .. n {
                    { // for split_at_mut
                        let (sorted,to_sort) = self.data[..].split_at_mut(i);
                        
                        let mut f_data = slot.drain();
                        let mut s_data = to_sort[0].filtered_drain();
                        let mut sorted = sorted.iter_mut(); 
                        
                        let mut f = f_data.next();
                        let mut s = s_data.next();
                        
                        loop {
                            while f.is_some() && s.is_some() {
                                let fe = f.take().unwrap(); // safe
                                let se = s.take().unwrap(); // safe
                                match fe.0 < se.0 {
                                    true => {
                                        self.tmp_merge_keys.push(fe.0);
                                        self.tmp_merge_values.push(fe.1);
                                        f = f_data.next();
                                        s = Some(se);
                                    },
                                    false => {
                                        self.tmp_merge_keys.push(se.0);
                                        self.tmp_merge_values.push(se.1);                           
                                        f = Some(fe);
                                        s = s_data.next();
                                    },
                                }
                            }
                            if f.is_none() {
                                // f_data finished, try to get next
                                match sorted.next() {
                                    Some(ms) => {
                                        f_data = ms.drain();
                                        f = f_data.next();
                                    },
                                    None => break, // all fs are done
                                }
                            } else {
                                // s is done
                                break;
                            }
                        }
                        if f.is_some() {
                            loop {
                                while let Some(fe) = f {
                                    self.tmp_merge_keys.push(fe.0);
                                    self.tmp_merge_values.push(fe.1);
                                    f = f_data.next();
                                }
                                match sorted.next() {
                                    Some(ms) => {
                                        f_data = ms.drain();
                                        f = f_data.next();
                                    },
                                    None => break, // all fs are done
                                }
                            }
                        } else {
                            while let Some(se) = s {
                                self.tmp_merge_keys.push(se.0);
                                self.tmp_merge_values.push(se.1);
                                s = s_data.next();
                            }
                        }
                    }
                    
                    // fs and s are done, spliting tmp_merge_* into previous slots
                    //   on all iters except last
                    if i < (n-1) {
                        let mut iter = self.tmp_merge_keys.drain(..).zip(self.tmp_merge_values.drain(..));
                        let mut ex = slot.fill_in(&mut iter);
                        for j in 0 ..= i {
                            ex = self.data[j].fill_in(&mut iter);
                            if ex { break; }
                        }
                        if !ex {
                            if let Some(_) = iter.next() {
                                return Err("merged data greater then the sum of the parts");
                            }
                        }
                    }
                }
            }
            for i in 0 .. n {
                self.data[i].clear();
            }
        }
        std::mem::swap(&mut self.data[n].keys, &mut self.tmp_merge_keys);
        std::mem::swap(&mut self.data[n].values, &mut self.tmp_merge_values);
   
        let c = self.data[n].keys.len();
        self.data[n].flags.set_ones(c);
        Ok(())
    }
}

#[cfg(feature = "debug")]
impl<K: Ord, V> CivMap<K,V> {
    pub fn check_len(&self) -> usize {
        self.slot.len() + self.data.iter().fold(0,|acc,x|acc+x.check_len())
    }
    pub fn max_capacity(&self) -> usize {
        self.slot.max_size() + self.data.iter().fold(0,|acc,x|acc+x.capacity)
    }
    pub fn real_capacity(&self) -> usize {
        self.slot.max_size() + self.data.iter().fold(0,|acc,x|acc+x.keys.capacity())
    }
    pub fn capacities(&self) -> Vec<usize> {
        self.data.iter().map(|ms|ms.keys.capacity()).collect()
    }
    pub fn statistics(&self) -> Vec<String> {
        let mut s = (0,0,0);
        let mut v = Vec::new();
        for (i,ms) in self.data.iter().enumerate() {
            if !ms.empty() {
                let len = ms.check_len();
                let cap = ms.capacity;
                let tombs = cap - len;
                v.push(format!("{:3}: {:12} {:12} {:12}",i,cap,len,tombs));
                s.0 += cap;
                s.1 += len;
                s.2 += tombs;
            }
        }
        v.push(format!("TOT: {:12} {:12} {:12}",s.0,s.1,s.2));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore]
    fn test_merge_sort_1() {
        let mut map: CivMap<u64,u32> = CivMap::new();
        map.slot = Slot::test(1);
        let test_data = vec![3,7,1,10,14,2,8,12,11,6,15,9,5,4,13].into_iter().map(|k|(k,k as u32)).collect::<Vec<_>>();
        for (k,v) in test_data {
            map.insert(k,v);
            println!("Size: {} ({})",map.len,map.tombs);
            println!("Slot: {:?}",map.slot);
            for (i,ms) in map.data.iter().enumerate() {
                println!("Data{:02}: {:?} -> {:?}",i,ms.keys,ms.values);
            }
            println!("");
        }
        panic!();
    }

    #[test]
    #[ignore]
    fn test_merge_sort_2() {
        let mut map: CivMap<u64,u32> = CivMap::new();
        map.slot = Slot::test(3);
        let test_data = vec![3,7,1,10,14,2,8,12,11,6,15,9,5,4,13].into_iter().map(|k|(k,k as u32)).collect::<Vec<_>>();
        for (k,v) in test_data {
            map.insert(k,v);
            println!("Size: {} ({})",map.len,map.tombs);
            println!("Slot: {:?}",map.slot);
            for (i,ms) in map.data.iter().enumerate() {
                println!("Data{:02}: {:?} -> {:?}",i,ms.keys,ms.values);
            }
            println!("");
        }
        for k in [4,8,5,11,7].iter() {
            map.remove(k);
        }
        
        map.insert(16,16);
        println!("Size: {} ({})",map.len,map.tombs);
        println!("Slot: {:?}",map.slot);
        for (i,ms) in map.data.iter().enumerate() {
            println!("Data{:02}: {:?} -> {:?}",i,ms.keys,ms.values);
        }
        println!("");
        panic!();
    }

    #[test]
    fn test_iter() {
        let cnt = 1_000_000;
        let mut res = Vec::with_capacity(cnt);
        let mut map: CivMap<u64,u32> = CivMap::new();
        for i in 0 .. cnt {
            map.insert(i as u64, i as u32);
            if i % 10 != 0 {
                res.push((i as u64, i as u32));
            }
        }
        for i in 0 .. cnt {
            if i % 10 == 0 {
                map.remove(&(i as u64));
            }
        }

        let mut lib = map.filtered_iter().map(|(k,v)| (*k,*v)).collect::<Vec<_>>();
        lib.sort_by_key(|(k,_)|*k);

        assert_eq!(res,lib);
    }
    
}
