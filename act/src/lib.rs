mod sh {
    include!(concat!(env!("OUT_DIR"), "/sh.rs"));
}

pub use sh::*;
