use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::models::User;

fn demo() {
    let _h: HashMap<String, String> = Default::default();
    let _b: BTreeMap<String, String> = Default::default();
    let _a: Arc<String> = Arc::new("x".into());
    let _s = Serialize::default();
    let _d = Deserialize::default();
    let _u: User = todo!();
}
