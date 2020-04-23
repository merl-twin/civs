
use crate::{Flags,Filled};

#[derive(Debug)]
struct Slot<Key>{
    flags: u64,
    data: Vec<Key>,
}
impl<Key> Slot<Key> {
    fn size() -> usize {
        64
    }
}
impl<Key> Slot<Key>
where Key: Ord
{
    fn new() -> Slot<Key> {
        Slot {
            flags: 0,
            data: Vec::with_capacity(64),
        }
    }
    fn contains(&self, k: &Key) -> (Option<usize>,Option<usize>) { // Key slot idx + Empty slot idx if known
        let mut fl = self.flags;
        let mut empty = None;
        for (i,v) in self.data.iter().enumerate() {
            if (fl & 0x1) > 0 {
                if v == k {
                    return (Some(i),empty);
                }
            } else {
                empty = Some(i);
            }
            fl >>= 1;
        }
        (None,empty)
    }
    fn insert(&mut self, k: Key) -> (bool,Filled) {
        let idx = match self.contains(&k) {
            (Some(_),_) => return (true,if self.data.len() >= 64 { Filled::Full } else { Filled::HasSlots }),
            (None,None) => {
                let idx = self.data.len();
                self.data.push(k);
                idx
            },
            (None,Some(idx)) => {
                self.data[idx] = k;
                idx
            }
        };
        self.flags |= 0x1u64 << idx;
        (false,if self.data.len() >= 64 { Filled::Full } else { Filled::HasSlots })
    }
    fn remove(&mut self, k: &Key) -> bool {
        match self.contains(&k) {
            (Some(idx),_) => {
                let msk = 0xFFFFFFFFFFFFFFFF - (0x1u64 << idx);
                self.flags &= msk;
                true
            },
            (None,_) => false,
        }
    }
    #[inline]
    fn clear(&mut self) {
        self.flags = 0;
        self.data.clear();
    }
    fn into_multislot(&mut self) -> MultiSlot<Key> {
        let mut v = Vec::with_capacity(self.flags.count_ones() as usize);
        let mut fl = self.flags;
        for k in self.data.drain(..) {
            if (fl & 0x1) > 0 {
                v.push(k);
            }
            fl >>= 1;
        }
        v.sort();
        self.clear();
        MultiSlot {
            _sz: 1,
            empty: false,
            flags: Flags::ones(v.len()),
            data: v,
        }
    }
}


struct MultiSlot<Key> {
    _sz: usize,
    empty: bool,
    flags: Flags,
    data: Vec<Key>,
}
impl<Key> MultiSlot<Key>
    where Key: Ord
{
    fn empty(sz: usize) -> MultiSlot<Key> {   
        MultiSlot {
            _sz: sz,
            empty: true,
            flags: Flags::nulls(Slot::<Key>::size() * (0x1 << (sz-1))),
            data: Vec::new(),
        }
    }
    fn contains(&self, k: &Key) -> Option<usize> {
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


pub struct CivSet<Key> {
    len: usize,
    _tombs: usize,
    slot: Slot<Key>,
    data: Vec<MultiSlot<Key>>,

    tmp_c: usize,
}
impl<Key: Ord> CivSet<Key> {
    pub fn new() -> CivSet<Key> {
        CivSet {
            len: 0,
            _tombs: 0,
            slot: Slot::new(),
            data: Vec::new(),

            tmp_c: 0,
        }
    }
    pub fn contains(&mut self, k: &Key) -> bool {
        match self.slot.contains(k) {
            (Some(_),_) => true,
            (None,_) => self.multy_contains(k).is_some(),
        }
    }
    fn multy_contains(&mut self, k: &Key) -> Option<(usize,usize)> {
        for (n,ms) in self.data.iter().enumerate() {
            if let Some(idx) = ms.contains(k) {
                return Some((n,idx));
            }
        }
        None
    }
    pub fn insert(&mut self, k: Key) -> bool {
        if self.multy_contains(&k).is_some() {
            return true;
        }
        let (r,filled) = self.slot.insert(k);
        if let Filled::Full = filled {
            if self.data.len() == 0 {
                self.data.push(self.slot.into_multislot());
            } else {
                let mut n = 0;
                while (n < self.data.len())&&(!self.data[n].empty) { n += 1; }
                if n == self.data.len() {
                    self.data.push(MultiSlot::empty(n+1));
                }
                if let Err(s) = self.merge_into(n) {
                    panic!("Unreachable merge_into: {}",s);
                }
            }
        }
        self.len += 1;
        r
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn remove(&mut self, k: &Key) -> bool {
        match self.multy_contains(&k) {
            Some((msi,idx)) => {
                self.data[msi].flags.unset(idx);
                true
            },
            None => {
                self.slot.remove(k)
            }
        }
    }
    fn merge_into(&mut self, n: usize) -> Result<(),&'static str> {
        if !self.data[n].empty { return Err("data[n] is not empty"); }
        let mut cnt = Slot::<Key>::size();
        for i in 0 .. n {
            if self.data[i].empty { return Err("one of data[0..n] is empty"); }
            cnt += self.data[i].data.len();
        }
        self.data[n].data.reserve(cnt);
        
        {
            let sl_flags = Flags(vec![self.slot.flags]);
            for (i,k) in self.slot.data.drain(..).enumerate() {
                if sl_flags.get(i) {
                    self.data[n].data.push(k);
                }
            }
            self.slot.clear();

            let mut tmp = Vec::new();
            let mut sl_flags = Flags::tmp();
            std::mem::swap(&mut self.data[n].data,&mut tmp);
            for i in 0 .. n {
                std::mem::swap(&mut self.data[i].flags,&mut sl_flags);
                for (j,k) in self.data[i].data.drain(..).enumerate() {
                    if sl_flags.get(j) {
                        tmp.push(k);
                    }
                }
                std::mem::swap(&mut self.data[i].flags,&mut sl_flags);
                self.data[i].clear();
            }
            std::mem::swap(&mut self.data[n].data,&mut tmp);

            self.data[n].data.sort();
            self.tmp_c += 1;
        }
        
        
        self.data[n].empty = false;
        let c = self.data[n].data.len();
        self.data[n].flags.set_ones(c);
        Ok(())
    }
}
