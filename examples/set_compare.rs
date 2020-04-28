use rand::Rng;

use collections::CivSet;
use std::collections::BTreeSet;

fn diff(func: &'static str, _x: u64, _set: &CivSet<u64>, _ctr: &BTreeSet<u64>) {
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
            println!("CivSet:   {}, {}",set.len(),set.tombs());
            println!("BTreeSet: {}\n",ctr.len());
            tm = std::time::Instant::now();
        }
    }
    
}
