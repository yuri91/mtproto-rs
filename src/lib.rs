extern crate byteorder;
#[macro_use]
extern crate error_chain;

mod errors {
    error_chain! {
        errors {
            Deserialize(t: String) {
                description("failed to deserialize")
                display("failed to deserialize: {}",t)
            }
            Serialize(t: String) {
                description("failed to serialize")
                display("failed to serialize: {}",t)
            }
        }
    }
}

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use std::io::{Write,Read};

use errors::*;

trait TLType : Sized {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()>;
    fn deserialize<R: Read>(input: &mut R) -> Result<Self>;
}

impl TLType for i32 {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()> {
        out.write_i32::<LittleEndian>(*self)
            .chain_err(||ErrorKind::Serialize("i32".into()))
    }
    fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
        input.read_i32::<LittleEndian>()
            .chain_err(||ErrorKind::Deserialize("i32".into()))
    }
}
impl TLType for i64 {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()>{
        out.write_i64::<LittleEndian>(*self)
            .chain_err(||ErrorKind::Serialize("i64".into()))
    }
    fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
        input.read_i64::<LittleEndian>()
            .chain_err(||ErrorKind::Deserialize("i64".into()))
    }
}
impl TLType for f64 {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()>{
        out.write_f64::<LittleEndian>(*self)
            .chain_err(||ErrorKind::Serialize("f64".into()))
    }
    fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
        input.read_f64::<LittleEndian>()
            .chain_err(||ErrorKind::Deserialize("f64".into()))
    }
}
// TODO: this is wrong. see doc for how to properly encode length
impl TLType for Vec<u8> {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()>{
        (self.len() as i32).serialize(out)?;
        out.write_all(self)
            .chain_err(||ErrorKind::Serialize("bytes".into()))
    }
    fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
        let size = i32::deserialize(input)? as usize;
        let mut bytes = vec![0;size];
        input.read_exact(&mut bytes[..])
            .chain_err(||ErrorKind::Deserialize("bytes".into()))?;
        Ok(bytes)
    }
}
impl TLType for String {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()>{
        self.clone().into_bytes().serialize(out)
            .chain_err(||ErrorKind::Serialize("string".into()))
    }
    fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
        let bytes = TLType::deserialize(input)
            .chain_err(||ErrorKind::Deserialize("string".into()))?;
        String::from_utf8(bytes)
            .chain_err(||ErrorKind::Deserialize("string".into()))
    }
}
impl<T> TLType for Vec<T> where T: TLType {
    fn serialize<W: Write>(&self,out: &mut W) -> Result<()>{
        const ID : i32 = 481674261;
        ID.serialize(out)?;
        (self.len() as i32).serialize(out)?;
        for elem in self {
            elem.serialize(out)?;
        }
        Ok(())
    }
    fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
        const ID : i32 = 481674261;
        let read_id = i32::deserialize(input)?;
        if ID != read_id {
            bail!(ErrorKind::Deserialize("vector".into()));
        }
        let size = i32::deserialize(input)? as usize;
        let mut vector = Vec::with_capacity(size);
        for _ in 0..size {
            vector.push(T::deserialize(input)?);
        }
        Ok(vector)
    }
}

include!(concat!(env!("OUT_DIR"), "/constructors.rs"));


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_i32() {
        let i = -49302394i32;
        let mut cur = Cursor::new(Vec::new());
        i.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let ii = i32::deserialize(&mut cur).unwrap();
        assert_eq!(i,ii);
    }
    #[test]
    fn test_i64() {
        let i = -493023949029i64;
        let mut cur = Cursor::new(Vec::new());
        i.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let ii = i64::deserialize(&mut cur).unwrap();
        assert_eq!(i,ii);
    }
    #[test]
    fn test_f64() {
        let f = -493023.949029f64;
        let mut cur = Cursor::new(Vec::new());
        f.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let ff = f64::deserialize(&mut cur).unwrap();
        assert_eq!(f,ff);
    }
    #[test]
    fn test_bytes() {
        let b = vec![1u8,2,3,4,5];
        let mut cur = Cursor::new(Vec::new());
        b.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let bb :Vec<u8> = Vec::deserialize(&mut cur).unwrap();
        assert_eq!(b,bb);
    }
    #[test]
    fn test_string() {
        let s = String::from("puppa");
        let mut cur = Cursor::new(Vec::new());
        s.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let ss  = String::deserialize(&mut cur).unwrap();
        assert_eq!(s,ss);
    }
    #[test]
    fn test_upload_file() {
        let f = upload::File::File {
            bytes: vec![1,2,3,4,5],
            kind: storage::FileType::FileGif{},
            mtime: -100
        };
        let mut cur = Cursor::new(Vec::new());
        f.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let ff  = upload::File::deserialize(&mut cur).unwrap();
        assert_eq!(f,ff);
    }
    #[test]
    fn test_wallpaper() {
        let w = WallPaper::WallPaper {
            sizes: vec![
                PhotoSize::PhotoSizeEmpty {kind: "empty".into()},
                PhotoSize::PhotoCachedSize {
                    w:10,
                    h:100,
                    kind: "cached".into(),
                    location: FileLocation::FileLocation {
                        local_id: 1,
                        dc_id:2,
                        secret: 10000000000,
                        volume_id: -1
                    },
                    bytes: vec![1,2,3,4]
                }
            ],
            title: "title".into(),
            color: 10,
            id: -133
        };
        let mut cur = Cursor::new(Vec::new());
        w.serialize(&mut cur).unwrap();
        cur.set_position(0);
        let ww  = WallPaper::deserialize(&mut cur).unwrap();
        assert_eq!(w,ww);
    }
}
