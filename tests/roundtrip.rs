use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq)]
struct A {
    name: String,
    age: u8,
    debt: i64,
}

#[test]
fn round_trip() -> muon::Result<()> {
    let a = A {
        name: "First, Last".to_string(),
        age: 21,
        debt: -5_000_000,
    };

    let s = muon::to_string(&a)?;
    let aa: A = muon::from_str(&s)?;

    assert!(a == aa);
    Ok(())
}
