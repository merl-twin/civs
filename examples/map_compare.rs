#[cfg(not(feature = "debug"))]
compile_error!("Feature 'debug' must be enabled for this example");

#[cfg(not(feature = "debug"))]
fn main() {}

#[cfg(feature = "debug")]
use {
    rand::Rng,
    civs::CivMap,
    std::collections::BTreeMap,
    std::io::Write,
};


#[cfg(feature = "debug")]
fn main() {
    fn diff(func: &'static str, _x: u64, _set: &CivMap<u64,u32>, _ctr: &BTreeMap<u64,u32>) {
        panic!("Maps differ in {}",func);
    }
    
    let mut rng = rand::thread_rng();
    
    let mut set = CivMap::new();
    let mut ctr = BTreeMap::new();

    let mut tm = std::time::Instant::now();
    let dur = std::time::Duration::new(300,0);
    
    loop {
        let x: u32 = rng.gen();
        let x = (x as u64) % 1_000_000_000;

        let civs_cont = set.get(&x);
        let ctrl_cont = ctr.get(&x);

        if civs_cont != ctrl_cont { diff("get", x,&set,&ctr); }

        match civs_cont {
            None => {
                let civs_cont = set.insert(x,x as u32);
                let ctrl_cont = ctr.insert(x,x as u32);
                if civs_cont != ctrl_cont { diff("insert", x,&set,&ctr); }
            },
            Some(_) => {
                let civs_cont = set.remove(&x).map(|ritem|ritem.copied());
                let ctrl_cont = ctr.remove(&x);
                if civs_cont != ctrl_cont { diff("remove", x,&set,&ctr); }
            },
        }

        if tm.elapsed() > dur {
            let checked = set.check_len();
            let real_capacity = set.real_capacity();
            let max_capacity = set.max_capacity();
            let tombs = set.tombs();
            println!("BTreeMap: {}",ctr.len());
            println!("CivMap:   {}; {}, {}; {}\n    tomb/data {:.3}, data/cap {:.3}, real/max {:.3}",checked,set.len(),tombs,real_capacity,tombs as f64/checked as f64,checked as f64/real_capacity as f64,real_capacity as f64/max_capacity as f64);

            writeln!(std::io::stderr(),"BTreeMap: {}",ctr.len()).unwrap();
            writeln!(std::io::stderr(),"CivMap:   {}; {}, {}; {}\n    tomb/data {:.3}, data/cap {:.3}, real/max {:.3}",checked,set.len(),tombs,real_capacity,tombs as f64/checked as f64,checked as f64/real_capacity as f64,real_capacity as f64/max_capacity as f64).unwrap();
            for s in set.statistics() {
                writeln!(std::io::stderr(),"{}",s).unwrap();
            }
            tm = std::time::Instant::now();
        }
    }
    
}
