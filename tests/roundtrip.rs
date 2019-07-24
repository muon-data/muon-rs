use muon_rs as muon;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct A {
    name: String,
    age: u8,
    debt: i64,
}

#[test]
fn derived_ser_de() -> muon::Result<()> {
    let a = A {
        name: "First, Last".to_string(),
        age: 21,
        debt: -5_000_000,
    };
    let s = muon::to_string(&a)?;
    let aa: A = muon::from_str(&s)?;
    assert_eq!(a, aa);
    Ok(())
}

#[test]
fn derived_de_ser() -> muon::Result<()> {
    let s = "name: Me, Myself\nage: 99\ndebt: -2\n";
    let a: A = muon::from_str(&s)?;
    let ss = muon::to_string(&a)?;
    assert_eq!(s, ss);
    Ok(())
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct B {
    people: Vec<C>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct C {
    name: String,
}

#[test]
fn dict_list() -> muon::Result<()> {
    let s = "people:\n   name: Genghis Khan\npeople:\n   name: Josef Stalin\npeople:\n   name: Dudley Doo-Right\n";
    let b: B = muon::from_str(&s)?;
    let ss = muon::to_string(&b)?;
    assert_eq!(ss, "people:\n  name: Genghis Khan\npeople:\n  name: Josef Stalin\npeople:\n  name: Dudley Doo-Right\n");
    Ok(())
}
