use anyhow::{Result, anyhow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnutuChunk {
    pub chunk_type: [u8; 4],
    pub payload: Vec<u8>,
}

impl AnutuChunk {
    pub fn new(chunk_type: [u8; 4], payload: Vec<u8>) -> Self {
        Self { chunk_type, payload }
    }

    pub fn total_size(&self) -> usize {
        12 + self.payload.len()
    }
}

pub struct ChunkSerializer;

impl ChunkSerializer {
    pub fn serialize_chunks(chunks: &[AnutuChunk]) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(1024);

        for chunk in chunks {
            buffer.extend_from_slice(&chunk.chunk_type);

            let length = chunk.payload.len() as u64;
            buffer.extend_from_slice(&length.to_le_bytes());

            buffer.extend_from_slice(&chunk.payload);
        }

        buffer
    }

    pub fn deserialize_chunks(data: &[u8]) -> Result<Vec<AnutuChunk>> {
        let mut chunks = Vec::new();
        let mut index = 0;
        let total_len = data.len();

        while index < total_len {
            if index + 12 > total_len {
                return Err(anyhow!(
                    "[ChunkSerializer] Corrupted .anutu binary: unexpected EOF in chunk header at offset {}",
                    index
                ));
            }

            let mut chunk_type = [0u8; 4];
            chunk_type.copy_from_slice(&data[index..index + 4]);
            index += 4;

            let mut length_bytes = [0u8; 8];
            length_bytes.copy_from_slice(&data[index..index + 8]);
            let length = u64::from_le_bytes(length_bytes) as usize;
            index += 8;

            if index + length > total_len {
                return Err(anyhow!(
                    "[ChunkSerializer] Corrupted .anutu binary: payload size ({}) exceeds remaining file bytes at offset {}",
                    length,
                    index
                ));
            }

            let payload = data[index..index + length].to_vec();
            index += length;

            chunks.push(AnutuChunk { chunk_type, payload });
        }

        Ok(chunks)
    }
}