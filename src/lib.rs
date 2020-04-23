

mod clones;

pub use crate::clones::{
    CivSet, CivMap,
};

#[derive(Debug)]
enum Filled {
    HasSlots,
    Full,
}

#[derive(Debug,Clone)]
struct Flags(Vec<u64>);
impl Flags {
    /*fn new64(sz: usize) -> Flags {
        let mut v = Vec::with_capacity(sz);
        for _ in 0 .. sz { v.push(0); }
        Flags(v)
}*/
    fn tmp() -> Flags {
        Flags(vec![])
    }
    fn nulls(sz: usize) -> Flags {
        let ln = sz/64 + 1;
        let mut v = Vec::with_capacity(ln);
        for _ in 0 .. ln { v.push(0); }
        Flags(v)
    }
    fn ones(sz: usize) -> Flags {
        let ln = sz/64 + 1;
        let mut v = Vec::with_capacity(ln);
        let mut s = sz;
        for _ in 0 .. ln {
            match s {
                0 => v.push(0),
                t @ _ if t < 64 => v.push(0xFFFFFFFFFFFFFFFFu64 >> (64 - t)),
                _ => v.push(0xFFFFFFFFFFFFFFFFu64),
            }
            if s > 64 { s -= 64; } else { s = 0; } 
        }
        while v.len() < ln { v.push(0); }
        Flags(v)
    }
    fn set_nulls(&mut self) {
        for i in 0 .. self.0.len() {
            self.0[i] = 0;
        }
    }
    fn set_ones(&mut self, sz: usize) {
        let ln = sz/64 + 1;
        while self.0.len() < ln { self.0.push(0); }
        let mut s = sz;
        for i in 0 .. ln {
            match s {
                0 => self.0[i] = 0,
                t @ _ if t < 64 => self.0[i] = 0xFFFFFFFFFFFFFFFFu64 >> (64 - t),
                _ => self.0[i] = 0xFFFFFFFFFFFFFFFFu64,
            }
            if s > 64 { s -= 64; } else { s = 0; } 
        }
    }
    #[inline]
    fn get(&self, idx: usize) -> bool {
        let i = idx/64;
        let j = idx%64;
        (self.0[i] & (0x1u64 << j)) > 0
    }
    #[inline]
    fn unset(&mut self, idx: usize) {
        let i = idx/64;
        let j = idx%64;
        self.0[i] &= 0xFFFFFFFFFFFFFFFFu64 - (0x1u64 << j);
    }
    /*#[inline]
    fn set(&mut self, idx: usize) {
        let i = idx/64;
        let j = idx%64;
        self.0[i] |= 0x1u64 << j;
    }*/
    /*fn clear(&mut self) {
        for v in &mut self.0 {
            *v = 0;
        }
    }*/
}


/*



struct SlotMap {
    data: BTreeSet<Key>,
}
impl SlotMap {
    fn new() -> SlotMap {
        SlotMap {
            data: BTreeSet::new(),
        }
    }
    fn size() -> usize {
        64*1024
    }
    fn contains(&self, k: &Key) -> bool {
        self.data.contains(k)
    }
    fn insert(&mut self, k: Key) -> (bool,Filled) {
        let r = self.data.insert(k);
        let f = if self.data.len() >= 64*1024 { Filled::Full } else { Filled::HasSlots };
        (r,f)
    }
    fn remove(&mut self, k: &Key) -> bool {
        self.data.remove(k)
    }
    fn clear(&mut self) {
        self.data.clear();
    }
    fn into_multislot(&mut self) -> MultiSlot {
        let mut v = Vec::with_capacity(self.data.len());
        for k in &self.data {
            v.push(*k);
        }
        self.clear();
        MultiSlot {
            sz: 1,
            empty: false,
            flags: Flags::ones(v.len()),
            data: v,
        }
    }
}

struct MultiSlotIterator<'t> {
    iter: std::iter::Enumerate<std::slice::Iter<'t,Key>>,
    flags: &'t Flags,
}
impl<'t> Iterator for MultiSlotIterator<'t> {
    type Item = &'t Key;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((n,k)) if self.flags.get(n) => break Some(k),
                Some(_) => continue,
                None => break None,
            }
        }
    }
}

impl<'t> IntoIterator for &'t MultiSlot {
    type Item = &'t Key;
    type IntoIter = MultiSlotIterator<'t>;
    fn into_iter(self) -> Self::IntoIter {
        MultiSlotIterator {
            iter: self.data.iter().enumerate(),
            flags: &self.flags,
        }
    }
}


struct FastSet2 {
    len: usize,
    tombs: usize,
    slot: SlotMap,
    data: Vec<MultiSlot>,
    tmp_c: usize,
}
impl FastSet2 {
    pub fn new() -> FastSet2 {
        FastSet2 {
            len: 0,
            tombs: 0,
            slot: SlotMap::new(),
            data: Vec::new(),
            tmp_c: 0,
        }
    }
    pub fn contains(&mut self, k: &Key) -> bool {
        match self.slot.contains(k) {
            true => true,
            false => self.multy_contains(k).is_some(),
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
                //if (n+1) < self.data.len() {
                    //self.relocate(n,n+1);
                //}
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
    /*fn relocate(&mut self, lower: usize, upper: usize) {
        let mut k = 0;
        let mut p = 0;
        let n = self.data[lower].data.len();
        while (k+p) < n {
            if p >= self.data[upper].data.len() { k += 1; } else {
                match self.data[lower].data[k] < self.data[upper].data[p] {
                    true => k += 1,
                    false => p += 1,
                }
            }
        }
        // k + p == n !!!
        for i in 0 .. p {
            let t = self.data[lower].data[k+i];
            self.data[lower].data[k+i] = self.data[upper].data[i];
            self.data[upper].data[i] = t;
            let b = self.data[lower].flags.get(k+i);
            match self.data[upper].flags.get(i) {
                true => self.data[lower].flags.set(k+i),
                false => self.data[lower].flags.unset(k+i),
            }
            match b {
                true => self.data[upper].flags.set(i),
                false => self.data[upper].flags.unset(i),
            }
        }
        self.data[lower].merge_sort(k);
        self.data[upper].merge_sort(p);
    }*/
    fn merge_into(&mut self, n: usize) -> Result<(),&'static str> {
        if !self.data[n].empty { return Err("data[n] is not empty"); }
        let mut cnt = Slot::size();
        for i in 0 .. n {
            if self.data[i].empty { return Err("one of data[0..n] is empty"); }
            cnt += self.data[i].data.len();
        }
        self.data[n].data.reserve(cnt);

        {
            for k in &self.slot.data {
                self.data[n].data.push(*k);
            }
            self.slot.clear();
            for i in 0 .. n {
                for j in 0 .. self.data[i].data.len() {
                    if self.data[i].flags.get(j) {
                        let k = self.data[i].data[j];
                        self.data[n].data.push(k);
                    }
                }
                self.data[i].clear();
            }

            self.data[n].data.sort();
            self.tmp_c += 1;
        }
        
        self.data[n].empty = false;
        let c = self.data[n].data.len();
        self.data[n].flags.set_ones(c);
        Ok(())
    }
}


 */
