use bytes::{Buf,BufMut,Bytes,BytesMut};

#[derive(Clone, Debug)]
pub struct NetworkFrame {
    pub message_type: u8,
    pub length: u32,
    pub payload: Bytes
}

impl NetworkFrame {
    pub fn to_bytes(&self) -> Bytes {  
        if self.message_type == 0 {
            return self.payload.clone();
        } else {
            let mut buffer = BytesMut::with_capacity(5 + self.payload.len());
            buffer.put_u8(self.message_type);
            buffer.put_u32(self.length);
            buffer.put(self.payload.clone());
            return buffer.freeze();
        }
    } 
}