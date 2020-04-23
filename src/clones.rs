
use crate::{Flags,Filled};

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

#[derive(Debug)]
struct Slot<K,V>{
    data: Vec<(K,V)>,
}
impl<K,V> Slot<K,V> {
    fn len(&self) -> usize {
        self.data.len()
    }
    fn max_size() -> usize {
        64
    }
}
impl<K: Ord,V> Slot<K,V> {
    fn new() -> Slot<K,V> {
        Slot {
            data: Vec::with_capacity(64),
        }
    }
    fn contains(&self, k: &K) -> Option<usize> { // Key slot idx 
        for (i,(ki,_)) in self.data.iter().enumerate() {
            if ki == k {
                return Some(i);
            }
        }
        None
    }
    fn get(&self, k: &K) -> Option<&V> {
        match self.contains(k) {
            Some(idx) => Some(&self.data[idx].1),
            None => None,
        }
    }
    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        match self.contains(k) {
            Some(idx) => Some(&mut self.data[idx].1),
            None => None,
        }
    }
    fn insert(&mut self, k: K, v: V) -> (Option<V>,Filled) {
        let opt_v = match self.contains(&k) {
            Some(idx) => {
                let mut tmp = v;
                std::mem::swap(&mut tmp, &mut self.data[idx].1);
                Some(tmp)
            },
            None => {
                self.data.push((k,v));
                None
            },
        };
        (opt_v,if self.data.len() >= 64 { Filled::Full } else { Filled::HasSlots })
    }
    fn remove(&mut self, k: &K) -> Option<V> {
        match self.contains(&k) {
            Some(idx) => Some(self.data.swap_remove(idx).1),
            None => None,
        }
    }
    #[inline]
    fn clear(&mut self) {
        self.data.clear();
    }
    fn into_map_multislot(&mut self) -> MapMultiSlot<K,V> {
        self.data.sort_by(|(k1,_),(k2,_)|k1.cmp(k2));
        let vc: Vec<(K,V)> = self.data.drain(..).collect();
        self.clear();
        MapMultiSlot {
            _sz: 1,
            empty: false,
            flags: Flags::ones(vc.len()),
            data: vc,
        }
    }
    fn into_set_multislot(&mut self) -> SetMultiSlot<K> {
        self.data.sort_by(|(k1,_),(k2,_)|k1.cmp(k2));
        let vc: Vec<K> = self.data.drain(..).map(|(k,_)|k).collect();
        self.clear();
        SetMultiSlot {
            _sz: 1,
            empty: false,
            flags: Flags::ones(vc.len()),
            data: vc,
        }
    }
}

struct SetMultiSlot<K> {
    _sz: usize,
    empty: bool,
    flags: Flags,
    data: Vec<K>,
}
impl<K: Ord> SetMultiSlot<K> {
    fn empty(sz: usize) -> SetMultiSlot<K> {   
        SetMultiSlot {
            _sz: sz,
            empty: true,
            flags: {
                let sz = Slot::<K,()>::max_size();
                Flags::nulls(sz * (0x1 << (sz-1)))
            },
            data: Vec::new(),
        }
    }
    fn contains(&self, k: &K) -> Option<usize> {
        if (self.data.len() == 0)||(*k < self.data[0])||(*k > self.data[self.data.len()-1]) { return None; }
        match self.data.binary_search(k) {
            Ok(idx) => match self.flags.get(idx) {
                true => Some(idx),
                false => None,
            },
            Err(_) => None,
        }
    }            
    fn clear(&mut self) {
        self.empty = true;
        self.flags.set_nulls();
        self.data.clear();
    }
}

struct MapMultiSlot<K,V> {
    _sz: usize,
    empty: bool,
    flags: Flags,
    data: Vec<(K,V)>,
}
impl<K: Ord, V> MapMultiSlot<K,V> {
    fn empty(sz: usize) -> MapMultiSlot<K,V> {   
        MapMultiSlot {
            _sz: sz,
            empty: true,
            flags: {
                let sz = Slot::<K,V>::max_size();
                Flags::nulls(sz * (0x1 << (sz-1)))
            },
            data: Vec::new(),
        }
    }
    fn contains(&self, k: &K) -> Option<usize> {
        if (self.data.len() == 0)||(*k < self.data[0].0)||(*k > self.data[self.data.len()-1].0) { return None; }
        match self.data.binary_search_by_key(&k,|(k,_)|k) {
            Ok(idx) => match self.flags.get(idx) {
                true => Some(idx),
                false => None,
            },
            Err(_) => None,
        }
    }            
    fn clear(&mut self) {
        self.empty = true;
        self.flags.set_nulls();
        self.data.clear();
    }
}
      
pub struct CivMap<K,V> {
    len: usize,
    _tombs: usize,
    slot: Slot<K,V>,
    data: Vec<MapMultiSlot<K,V>>,

    tmp_c: usize,
    tmp_merge_vec: Vec<(K,V)>,
    tmp_merge_flags: Flags,
}
       
impl<K: Ord, V> CivMap<K,V> {
    pub fn new() -> CivMap<K,V> {
        CivMap {
            len: 0,
            _tombs: 0,
            slot: Slot::new(),
            data: Vec::new(),

            tmp_c: 0,
            tmp_merge_vec: Vec::new(),
            tmp_merge_flags: Flags::tmp(),
        }
    }
    pub fn contains(&mut self, k: &K) -> bool {
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
                Some((msi,idx)) => Some(&self.data[msi].data[idx].1),
                None => None,
            }
        }
    }
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        match self.multy_contains(k) {
            Some((msi,idx)) => Some(&mut self.data[msi].data[idx].1),
            None => self.slot.get_mut(k),
        }
    }
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if let Some((msi,idx)) = self.multy_contains(&k) {
            let mut tmp = v;
            std::mem::swap(&mut tmp, &mut self.data[msi].data[idx].1);
            return Some(tmp);
        }
        let (r,filled) = self.slot.insert(k,v);
        if let Filled::Full = filled {
            if self.data.len() == 0 {
                self.data.push(self.slot.into_map_multislot());
            } else {
                let mut n = 0;
                while (n < self.data.len())&&(!self.data[n].empty) { n += 1; }
                if n == self.data.len() {
                    self.data.push(MapMultiSlot::empty(n+1));
                }
                if let Err(s) = self.merge_into(n) {
                    panic!("Unreachable merge_into: {}",s);
                }
                // TODO: Check size (cause of removed) 
            }
        }
        self.len += 1;
        r
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn remove(&mut self, k: &K) -> Option<RemovedItem<V>> {
        match self.multy_contains(&k) {
            Some((msi,idx)) => {
                self.data[msi].flags.unset(idx);
                Some(RemovedItem::Ref(&mut self.data[msi].data[idx].1))
            },
            None => match self.slot.remove(k) {
                Some(v) => Some(RemovedItem::Owned(v)),
                None => None,
            },
        }
    }
    fn merge_into(&mut self, n: usize) -> Result<(),&'static str> {
        if !self.data[n].empty { return Err("data[n] is not empty"); }
        let mut cnt = self.slot.len();
        for i in 0 .. n {
            if self.data[i].empty { return Err("one of data[0..n] is empty"); }
            cnt += self.data[i].data.len();
        }
        self.data[n].data.reserve(cnt);
        
        {
            for p in self.slot.data.drain(..) {
                self.data[n].data.push(p);
            }
            self.slot.clear();

            std::mem::swap(&mut self.data[n].data, &mut self.tmp_merge_vec);
            for i in 0 .. n {
                std::mem::swap(&mut self.data[i].flags,&mut self.tmp_merge_flags);
                for (j,k) in self.data[i].data.drain(..).enumerate() {
                    if self.tmp_merge_flags.get(j) {
                        self.tmp_merge_vec.push(k);
                    }
                }
                std::mem::swap(&mut self.data[i].flags,&mut self.tmp_merge_flags);
                self.data[i].clear();
            }
            std::mem::swap(&mut self.data[n].data, &mut self.tmp_merge_vec);

            self.data[n].data.sort_by(|(k1,_),(k2,_)|k1.cmp(k2));
            self.tmp_c += 1;
        }
        
        
        self.data[n].empty = false;
        let c = self.data[n].data.len();
        self.data[n].flags.set_ones(c);
        Ok(())
    }
}



pub struct CivSet<K> {
    len: usize,
    _tombs: usize,
    slot: Slot<K,()>,
    data: Vec<SetMultiSlot<K>>,

    tmp_c: usize,
    tmp_merge_vec: Vec<K>,
    tmp_merge_flags: Flags,
}
       
impl<K: Ord> CivSet<K> {
    pub fn new() -> CivSet<K> {
        CivSet {
            len: 0,
            _tombs: 0,
            slot: Slot::new(),
            data: Vec::new(),

            tmp_c: 0,
            tmp_merge_vec: Vec::new(),
            tmp_merge_flags: Flags::tmp(),
        }
    }
    pub fn contains(&mut self, k: &K) -> bool {
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
    pub fn insert(&mut self, k: K) -> bool {
        if self.multy_contains(&k).is_some() {
            return true;
        }
        let (r,filled) = self.slot.insert(k,());
        if let Filled::Full = filled {
            if self.data.len() == 0 {
                self.data.push(self.slot.into_set_multislot());
            } else {
                let mut n = 0;
                while (n < self.data.len())&&(!self.data[n].empty) { n += 1; }
                if n == self.data.len() {
                    self.data.push(SetMultiSlot::empty(n+1));
                }
                if let Err(s) = self.merge_into(n) {
                    panic!("Unreachable merge_into: {}",s);
                }
                // TODO: Check size (cause of removed) 
            }
        }
        self.len += 1;
        r.is_some()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn remove(&mut self, k: &K) -> bool {
        match self.multy_contains(&k) {
            Some((msi,idx)) => {
                self.data[msi].flags.unset(idx);
                true
            },
            None => self.slot.remove(k).is_some(),
        }
    }
    fn merge_into(&mut self, n: usize) -> Result<(),&'static str> {
        if !self.data[n].empty { return Err("data[n] is not empty"); }
        let mut cnt = self.slot.len();
        for i in 0 .. n {
            if self.data[i].empty { return Err("one of data[0..n] is empty"); }
            cnt += self.data[i].data.len();
        }
        self.data[n].data.reserve(cnt);
        
        {
            for p in self.slot.data.drain(..) {
                self.data[n].data.push(p.0);
            }
            self.slot.clear();

            std::mem::swap(&mut self.data[n].data, &mut self.tmp_merge_vec);
            for i in 0 .. n {
                std::mem::swap(&mut self.data[i].flags,&mut self.tmp_merge_flags);
                for (j,k) in self.data[i].data.drain(..).enumerate() {
                    if self.tmp_merge_flags.get(j) {
                        self.tmp_merge_vec.push(k);
                    }
                }
                std::mem::swap(&mut self.data[i].flags,&mut self.tmp_merge_flags);
                self.data[i].clear();
            }
            std::mem::swap(&mut self.data[n].data, &mut self.tmp_merge_vec);

            self.data[n].data.sort();
            self.tmp_c += 1;
        }
        
        
        self.data[n].empty = false;
        let c = self.data[n].data.len();
        self.data[n].flags.set_ones(c);
        Ok(())
    }
}
        
