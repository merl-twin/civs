
use crate::{
    Flags,Filled,
    civs::Slot,
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

pub(crate) struct MapMultiSlot<K,V> {
    _sz: usize,
    empty: bool,
    flags: Flags,
    keys: Vec<K>,
    values: Vec<V>,
}
impl<K: Ord, V> MapMultiSlot<K,V> {
    pub(crate) fn new(data: Vec<(K,V)>) -> MapMultiSlot<K,V> {
        let mut keys = Vec::with_capacity(data.len());
        let mut values = Vec::with_capacity(data.len());
        for (k,v) in data {
            keys.push(k);
            values.push(v);
        }
        MapMultiSlot {
            _sz: 1,
            empty: data.len() == 0,
            flags: Flags::ones(data.len()),
            keys: keys,
            values: values,
        }
    }
    fn empty(sz: usize) -> MapMultiSlot<K,V> {   
        MapMultiSlot {
            _sz: sz,
            empty: true,
            flags: {
                let sz = Slot::<K,V>::max_size();
                Flags::nulls(sz * (0x1 << (sz-1)))
            },
            keys: Vec::new(),
            values: Vec::new(),
        }
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
        self.empty = true;
        self.flags.set_nulls();
        self.keys.clear();
        self.values.clear();
    }
    fn reserve(&mut self, cnt: usize) {
        self.keys.reserve(cnt);
        self.values.reserve(cnt);
    }
}
      
pub struct CivMap<K,V> {
    len: usize,
    tombs: usize,
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
            tombs: 0,
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
    pub fn tombs(&self) -> usize {
        self.tombs
    }
    pub fn remove(&mut self, k: &K) -> Option<RemovedItem<V>> {
        match self.multy_contains(&k) {
            Some((msi,idx)) => {
                self.tombs += 1;
                self.data[msi].flags.unset(idx);
                Some(RemovedItem::Ref(&mut self.data[msi].values[idx]))
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
        self.data[n].reserve(cnt);
        
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

