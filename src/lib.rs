/*
MIT License

Copyright (c) 2017 Joshua Karns

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/
extern crate cfile_rs;

use std::io::Error;
use cfile_rs::CFile;
use std::io::SeekFrom;
use std::io::Write;
use std::io::Seek;
use std::slice;
use std::io::Read;
use std::path::Path;
use std::mem;

static SIZE_OF_U64: usize = 8;
static SIZE_OF_U32: usize = 4;
static SIZE_OF_U16: usize = 2;
static SIZE_OF_U8:  usize = 1;
static SIZE_OF_I64: usize = 8;
static SIZE_OF_I32: usize = 4;
static SIZE_OF_I16: usize = 2;
static SIZE_OF_I8:  usize = 1;

pub trait RandomAccessFile : Sized {
    fn new(path: &str) -> Result<Self, Error>;
    fn read_at(&mut self, at: usize, dat: &mut [u8]) -> Result<usize, Error>;
    fn write_at(&mut self, at: usize, dat: &[u8]) -> Result<usize, Error>;
    fn append(&mut self, dat: &[u8]) -> Result<(), Error>;
    fn at(&mut self, index: usize) -> Result<u8, Error> {
        let x = &mut [0u8];
        match self.read_at(index, x) {
            Ok(_) => Ok(x[0]),
            Err(e) => Err(e)
        }
    }
}

impl RandomAccessFile for CFile {
    fn new(path: &str) -> Result<CFile, Error> {
        CFile::open_random_access(path)
    }

    fn read_at(&mut self, at: usize, dat: &mut [u8]) -> Result<usize, Error> {
        let _ = self.seek(SeekFrom::Start(at as u64));
        self.read(dat)
    }

    fn write_at(&mut self, at: usize, data: &[u8]) -> Result<usize, Error> {
        let _ = self.seek(SeekFrom::Start(at as u64));
        self.write(data)
    }

    fn append(&mut self, data: &[u8]) -> Result<(), Error> {
        let _ = self.seek(SeekFrom::End(0));
        match self.write_all(data) {
            Ok(()) => {
                Ok(())
            },
            Err(e) => {
                Err(e)
            }
        }
    }
}

pub trait Serialize where Self: Sized {
    type DeserializeOutput: Sized;
    fn serialize(&self, to: &mut Write) -> Result<(), Error>;
    fn deserialize(from: &mut Read) -> Result<Self::DeserializeOutput, Error>;
    fn serialized_len(&self) -> u64;
}

macro_rules! serialize_primitive {
    ( $prim:ty, $size:expr ) => (
        impl Serialize for $prim {
            type DeserializeOutput = $prim;
            fn deserialize(from: &mut Read) -> Result<Self, Error> {
                let mut buffer = vec![0u8; $size];

                match from.read_exact(&mut buffer) {
                    Ok(_) => {
                        let t = unsafe {
                            slice::from_raw_parts((&buffer).as_ptr() as *const $prim, 1)
                        };
                        Ok(t[0])
                    },
                    Err(e) => Err(e)
                }
            }
            fn serialize(&self, to: &mut Write) -> Result<(), Error> {
                let x = [*self];
                let y = unsafe { slice::from_raw_parts((&x).as_ptr() as *const u8, $size) };
                if let Err(e) = to.write_all(y) {
                    Err(e)
                } else {
                    Ok(())
                }
            }
            fn serialized_len(&self) -> u64 {
                $size as u64
            }
        }
        impl Serialize for Vec<$prim> {
            type DeserializeOutput = Vec<$prim>;
            fn deserialize(from: &mut Read) -> Result<Self, Error> {
                let size: u64;
                match u64::deserialize(from) {
                    Ok(x) => {
                        size = x;
                    },
                    Err(e) => return Err(e)
                };
                type S = $prim;
                let mut ret = Vec::with_capacity(size as usize);
                for _ in 0..size {
                    match S::deserialize(from) {
                        Ok(x) => ret.push(x),
                        Err(e) => return Err(e)
                    };
                }
                Ok(ret)
            }
            fn serialize(&self, to: &mut Write) -> Result<(), Error> {
                match (self.len() as u64).serialize(to) {
                    Err(e) => return Err(e),
                    Ok(_) => ()
                };
                let y = unsafe { slice::from_raw_parts(self.as_ptr() as *const u8, $size * self.len()) };
                if let Err(e) = to.write_all(y) {
                    Err(e)
                } else {
                    Ok(())
                }
            }
            fn serialized_len(&self) -> u64 {
                (self.len() * $size + 8) as u64
            }
        }

        impl<'b> Serialize for &'b [$prim] {
            type DeserializeOutput = Vec<$prim>;
            fn deserialize(from: &mut Read) -> Result<Self::DeserializeOutput, Error> {
                let size: u64;
                match u64::deserialize(from) {
                    Ok(x) => {
                        size = x;
                    },
                    Err(e) => return Err(e)
                };
                type S = $prim;
                let mut ret = Vec::with_capacity(size as usize);
                for _ in 0..size {
                    match S::deserialize(from) {
                        Ok(x) => ret.push(x),
                        Err(e) => return Err(e)
                    };
                }
                Ok(ret)
            }
            fn serialize(&self, to: &mut Write) -> Result<(), Error> {
                match (self.len() as u64).serialize(to) {
                    Err(e) => return Err(e),
                    Ok(_) => ()
                };
                let y = unsafe { slice::from_raw_parts((*self).as_ptr() as *const u8, $size * self.len()) };
                if let Err(e) = to.write_all(&y) {
                    Err(e)
                } else {
                    Ok(())
                }
            }
            fn serialized_len(&self) -> u64 {
                (self.len() * $size + 8) as u64
            }
        }
    )
}

serialize_primitive!(i8,  SIZE_OF_I8);
serialize_primitive!(u64, SIZE_OF_U64);
serialize_primitive!(usize, mem::size_of::<usize>());
serialize_primitive!(u8,  SIZE_OF_U8);


serialize_primitive!(i16, SIZE_OF_I16);
serialize_primitive!(i32, SIZE_OF_I32);
serialize_primitive!(i64, SIZE_OF_I64);
serialize_primitive!(u16, SIZE_OF_U16);
serialize_primitive!(u32, SIZE_OF_U32);

serialize_primitive!(f32, SIZE_OF_U32);
serialize_primitive!(f64, SIZE_OF_U64);

impl Serialize for String {
    type DeserializeOutput = String;
    fn serialize(&self, from: &mut Write) -> Result<(), Error> {
        self.as_bytes().serialize(from)
    }
    fn deserialize(to: &mut Read) -> Result<Self, Error> {
        match Vec::<u8>::deserialize(to) {
            Ok(ret) => {
                Ok(String::from_utf8_lossy(&ret).into_owned())
            },
            Err(e) => Err(e)
        }
    }
    fn serialized_len(&self) -> u64 {
        (self.len() + 8) as u64
    }
}

impl<'a> Serialize for &'a str {
    type DeserializeOutput = String;
    fn serialize(&self, from: &mut Write) -> Result<(), Error> {
        self.as_bytes().serialize(from)
    }
    fn deserialize(to: &mut Read) -> Result<String, Error> {
        match Vec::<u8>::deserialize(to) {
            Ok(ret) => {
                Ok(String::from_utf8_lossy(&ret).into_owned())
            },
            Err(e) => Err(e)
        }
    }
    fn serialized_len(&self) -> u64 {
        (self.len() + 8) as u64
    }
}

/// TODO: Better tests.
#[cfg(test)]
mod tests {
    use Serialize;
    use RandomAccessFile;
    use cfile_rs;
    use cfile_rs::CFile;
    use std::io::SeekFrom;
    use std::io::Seek;
    #[test]
    fn it_works() {
        let mut raf: CFile = RandomAccessFile::new("test.txt").unwrap();
        65u64.serialize(&mut raf);
        raf.seek(SeekFrom::Start(0));
        let mut t = u64::deserialize(&mut raf).unwrap();
        assert!(t == 65)
    }
}
