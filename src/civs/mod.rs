use crate::Filled;

pub(crate) mod set;
pub(crate) mod map;

use set::SetMultiSlot;
use map::MapMultiSlot;


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
        MapMultiSlot::new(vc)
    }
    fn into_set_multislot(&mut self) -> SetMultiSlot<K> {
        self.data.sort_by(|(k1,_),(k2,_)|k1.cmp(k2));
        let vc: Vec<K> = self.data.drain(..).map(|(k,_)|k).collect();
        self.clear();
        SetMultiSlot::new(vc)
    }
}





        