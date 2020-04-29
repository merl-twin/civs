use rand::Rng;

use collections::CivSet;
use std::collections::BTreeSet;

use std::io::Write;

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
            let real_capacity = set.real_capacity();
            let max_capacity = set.max_capacity();
            let tombs = set.tombs();
            println!("BTreeSet: {}",ctr.len());
            println!("CivSet:   {}; {}, {}; {}\n    tomb/data {:.3}, data/cap {:.3}, real/max {:.3}",checked,set.len(),tombs,real_capacity,tombs as f64/checked as f64,checked as f64/real_capacity as f64,real_capacity as f64/max_capacity as f64);

            writeln!(std::io::stderr(),"BTreeSet: {}",ctr.len()).unwrap();
            writeln!(std::io::stderr(),"CivSet:   {}; {}, {}; {}\n    tomb/data {:.3}, data/cap {:.3}, real/max {:.3}",checked,set.len(),tombs,real_capacity,tombs as f64/checked as f64,checked as f64/real_capacity as f64,real_capacity as f64/max_capacity as f64).unwrap();
            for s in set.statistics() {
                writeln!(std::io::stderr(),"{}",s).unwrap();
            }
            tm = std::time::Instant::now();
        }
    }
    
}
