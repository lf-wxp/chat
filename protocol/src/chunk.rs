///
/// limitation of every chunk is 16k;
///
/// chunk type
/// 0 route mapping
/// 1 data chunk
///
/// route content
/// 0 direct
/// 1 mapping
///
/// chunk content
/// 0 text
/// 1 image
/// 2 file
/// 3 voice
/// 4 video
///
/// destination
/// like "Yo1Tr9F3iF-LFHX9i9GvA". Every char is deemed as ASCII encoding,
/// including 8 bits for single char,  so the 22 chars need  168 bits(21 * 8)
///
/// 16k = 131072 bits
///
/// all data is encoded by ascii, except media name with utf8.
///
/// route chunk
///
/// +------------+-------------+--------------+
/// | chunk type |  requestId  | mapping data |
/// +------------+-------------+--------------+
/// |   1 bit    |  168 bits   |  left bits   |
/// +------------+-------------+--------------+
///
/// data chunk
///
///                                                                                                          +---------------------------+
///                                                                                                          |for non-text type,max 500m |
///                                                                                                          | and first chunk.          |
///                                                                                                          |utf8 encoding|             |
/// +------------+-------------+------------+-------------+--------------+--------------------+--------------+-------------+-------------+-------------+
/// | chunk type |  requestId  | route type |   route     |   sequence   | total chunks count | content type |  media name | media size  |   content   |
/// +------------+-------------+---- -------+-------------+--------------+--------------------+--------------+-------------+-------------+-------------+
/// |   1 bit    |  168 bits   |   1 bit    |  176 bits   |   16 bits    |       16 bits      |    3 bits    |  400 bits   |   34 bits   |  left bits  |
/// +------------+-------------+------------+-------------+--------------+--------------------+--------------+-------------+-------------+-------------+
///
///
///
///
use bit_vec::BitVec;
use encoding_rs::*;
use std::ops::Range;

// 16k = 131072 bits
const MAX: usize = 131072;
const CHUNK_TYPE_RANGE: Range<usize> = 0..1;
const REQUEST_ID_RANGE: Range<usize> = 1..169;

struct RouteChunk {
  data: BitVec,
  max: usize,
}

impl RouteChunk {
  pub fn new(request_id: String, mapping: Vec<String>) -> Self {
    let mut chunk_type = BitVec::new();
    chunk_type.insert(0, false);
    let mut request_id = BitVec::from_bytes(request_id.as_bytes());
    let mapping = bincode::serialize(&mapping).unwrap();
    let mut mapping = BitVec::from_bytes(&mapping);
    let mut data = BitVec::new();
    data.append(&mut chunk_type);
    data.append(&mut request_id);
    data.append(&mut mapping);
    if data.len() > MAX {
      panic!("data size is over {} ", MAX);
    }
    Self { data, max: MAX }
  }
  fn get_mapping(&self) -> Vec<String> {
    let bv = self.get_bitvec_from(REQUEST_ID_RANGE.count() + 1);
    bincode::deserialize::<Vec<String>>(&bv.to_bytes()).unwrap()
  }
}

impl Chunk for RouteChunk {
  fn get_data(&self) -> &BitVec {
    &self.data
  }
}

pub trait Chunk {
  fn get_data(&self) -> &BitVec;
  fn get_request_id(&self) -> String {
    let bv = self.get_bitvec_range(REQUEST_ID_RANGE);
    let binding = bv.to_bytes();
    let (val, _, _) = UTF_8.decode(&binding);
    val.to_string()
  }
  fn get_bitvec_from(&self, start: usize) -> BitVec {
    let bv = self.get_data();
    let end = bv.len();
    self.get_bitvec_range(start..end)
  }
  fn get_bitvec_range(&self, range: Range<usize>) -> BitVec {
    let bv = self.get_data();
    bv.iter()
      .enumerate()
      .filter(|&(i, _)| range.contains(&i))
      .map(|(_, b)| b)
      .collect()
  }
  fn get_chunk_type(&self) -> u8 {
    let bv = self.get_bitvec_range(CHUNK_TYPE_RANGE);
    let mut num = 0;
    for bit in bv.iter() {
      num <<= 1; // 左移一位
      num |= bit as u8; // 添加当前位
    }
    num
  }
}

struct DataChunk {
  data: BitVec,
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn testing_route_chunk() {
    let request_id = "Yo1Tr9F3iF-LFHX9i9GvA".to_string();
    let mapping = vec![
      "Yo1Tr9F3iF-LFHX9i9GvA".to_string(),
      "Yo1Tr9F3iF-LFHX9i9GvA".to_string(),
      "Yo1Tr9F3iF-LFHX9i9GvA".to_string(),
    ];
    let route = RouteChunk::new(request_id.clone(), mapping.clone());
    let chunk_type = route.get_chunk_type();
    let id = route.get_request_id();
    let mapping_data = route.get_mapping();
    assert_eq!(chunk_type, 0);
    assert_eq!(id, request_id);
    assert_eq!(mapping_data, mapping);
  }
}
