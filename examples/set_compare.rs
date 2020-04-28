use rand::Rng;

use collections::CivSet;
use std::collections::BTreeSet;

fn diff(func: &'static str, _x: u64, set: &CivSet<u64>, ctr: &BTreeSet<u64>) {
    println!("CivSet:   {:?}",set);
    println!("BTreeSet: {:?}",ctr);
    panic!("Sets differ in {}",func);
}

fn main() {
    let mut rng = rand::thread_rng();
    
    let mut set = CivSet::new();
    let mut ctr = BTreeSet::new();

    let mut tm = std::time::Instant::now();
    let dur = std::time::Duration::new(300,0);
    
    loop {
        let x: u32 = rng.gen();
        let x = (x as u64) % 1_000_000_000;

        let civs_cont = set.contains(&x);
        let ctrl_cont = ctr.contains(&x);

        if civs_cont != ctrl_cont { diff("contains", x,&set,&ctr); }

        match civs_cont {
            false => {
                let civs_cont = set.insert(x);
                let ctrl_cont = ctr.insert(x);
                if civs_cont != ctrl_cont { diff("insert", x,&set,&ctr); }
            },
            true => {
                let civs_cont = set.remove(&x);
                let ctrl_cont = ctr.remove(&x);
                if civs_cont != ctrl_cont { diff("remove", x,&set,&ctr); }
            },
        }

        if tm.elapsed() > dur {
            let checked = set.check_len();
            let capacity = set.capacity();
            println!("CivSet:   {}; {}, {}, {}; {} {:.3}",checked,set.len(),set.tombs(),set.len()-set.tombs(),capacity,checked as f64/capacity as f64);
            println!("BTreeSet: {}\n",ctr.len());
            tm = std::time::Instant::now();
        }
    }
    
}
