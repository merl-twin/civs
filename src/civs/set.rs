
use crate::{
    Flags,Filled,
    civs::Slot,
};

pub(crate) struct SetMultiSlot<K> {
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
    pub(crate) fn new(data: Vec<K>) -> SetMultiSlot<K> {
        SetMultiSlot {
            _sz: 1,
            empty: data.len() == 0,
            flags: Flags::ones(data.len()),
            data: data,
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

pub struct CivSet<K> {
    len: usize,
    tombs: usize,
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
    pub fn tombs(&self) -> usize {
        self.tombs
    }
    pub fn remove(&mut self, k: &K) -> bool {
        match self.multy_contains(&k) {
            Some((msi,idx)) => {
                self.tombs += 1;
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