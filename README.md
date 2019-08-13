## muon-rs

A Rust library for the [MuON](https://github.com/muon-data/muon) data format,
using [serde](https://serde.rs).

See [documentation](https://docs.rs/muon-rs) for more information.

## Deserializing

The easiest way to deserialize data is to derive [`serde::Deserialize`] on
a struct.  Then use one of the [`from_`] functions.

[`serde::Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
[`from_`]: https://docs.rs/muon-rs/latest/muon_rs/index.html#functions

### Example

MuON file:
```muon
book: Pale Fire
  author: Vladimir Nabokov
  year: 1962
  character: John Shade
    location: New Wye
  character: Charles Kinbote
    location: Zembla
book: The Curious Incident of the Dog in the Night-Time
  author: Mark Haddon
  year: 2003
  character: Christopher Boone
    location: Swindon
  character: Siobhan
```

Rust code:
```rust
#[derive(Debug, Deserialize, Serialize)]
struct BookList {
    book: Vec<Book>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Book {
    title: String,
    author: String,
    year: Option<i16>,
    character: Vec<Character>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Character {
    name: String,
    location: Option<String>,
}

let muon = File::open("tests/books.muon")?;
let books: BookList = muon_rs::from_reader(muon)?;
println!("{:?}", books);
```

## Serializing

Deriving [`serde::Serialize`] on a struct is just as easy.  The [`to_`]
functions are used to serialize MuON data.

[`serde::Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
[`to_`]: https://docs.rs/muon-rs/latest/muon_rs/index.html#functions

### Example

```rust
let books = BookList {
    book: vec![
        Book {
            title: "Flight".to_string(),
            author: "Sherman Alexie".to_string(),
            year: Some(2007),
            character: vec![
                Character {
                    name: "Zits".to_string(),
                    location: Some("Seattle".to_string()),
                },
                Character {
                    name: "Justice".to_string(),
                    location: None,
                },
            ],
        },
    ],
};
let muon = muon_rs::to_string(&books)?;
println!("{:?}", muon);
```

## Types

MuON types can be mapped to different Rust types.

<table>
  <tr>
    <th>MuON Type</th>
    <th>Rust Types</th>
  </tr>
  <tr>
    <td>text</td>
    <td><a href="https://doc.rust-lang.org/std/string/struct.String.html">
        String</a>
    </td>
  </tr>
  <tr>
    <td>bool</td>
    <td><a href="https://doc.rust-lang.org/std/primitive.bool.html">bool</a>
    </td>
  </tr>
  <tr>
    <td>int</td>
    <td><a href="https://doc.rust-lang.org/std/primitive.i8.html">i8</a>
       <a href="https://doc.rust-lang.org/std/primitive.i16.html">i16</a>
       <a href="https://doc.rust-lang.org/std/primitive.i32.html">i32</a>
       <a href="https://doc.rust-lang.org/std/primitive.i64.html">i64</a>
       <a href="https://doc.rust-lang.org/std/primitive.i128.html">i128</a>
       <a href="https://doc.rust-lang.org/std/primitive.u8.html">u8</a>
       <a href="https://doc.rust-lang.org/std/primitive.u16.html">u16</a>
       <a href="https://doc.rust-lang.org/std/primitive.u32.html">u32</a>
       <a href="https://doc.rust-lang.org/std/primitive.u64.html">u64</a>
       <a href="https://doc.rust-lang.org/std/primitive.u128.html">u128</a>
    </td>
  </tr>
  <tr>
    <td>number</td>
    <td><a href="https://doc.rust-lang.org/std/primitive.f32.html">f32</a>
       <a href="https://doc.rust-lang.org/std/primitive.f64.html">f64</a>
    </td>
  </tr>
  <tr>
    <td>datetime</td>
    <td><a href="https://docs.rs/muon_rs/latest/muon-rs/struct.DateTime.html">DateTime</a></td>
  </tr>
  <tr>
    <td>date</td>
    <td><a href="https://docs.rs/muon_rs/latest/muon-rs/struct.Date.html">Date</a></td>
  </tr>
  <tr>
    <td>time</td>
    <td><a href="https://docs.rs/muon_rs/latest/muon-rs/struct.Time.html">Time</a></td>
  </tr>
  <tr>
    <td>record</td>
    <td>struct implementing
        <a href="https://docs.serde.rs/serde/trait.Deserialize.html">
        Deserialize</a>
    </td>
  </tr>
  <tr>
    <td>dictionary</td>
    <td><a href="https://doc.rust-lang.org/std/collections/struct.HashMap.html">
        HashMap</a>
    </td>
  </tr>
  <tr>
    <td>any</td>
    <td><a href="https://docs.rs/muon_rs/latest/muon-rs/enum.Value.html">Value</a></td>
  </tr>
</table>

### Contributing

Any feedback, bug reports or enhancement requests are welcome!
Please create an [issue](https://github.com/muon-data/muon-rs/issues) and join
the fun.
