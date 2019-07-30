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
struct Groups {
    group: Vec<Group>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Group {
    label: String,
    person: Vec<Person>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Person {
    name: String,
    born: u32,
    birthplace: Option<String>,
}

#[test]
fn people() -> muon::Result<()> {
    let g: Groups = muon::from_str(&include_str!("people.muon"))?;
dbg!(&g);
    assert_eq!(muon::to_string(&g)?, include_str!("people2.muon"));
    Ok(())
}
