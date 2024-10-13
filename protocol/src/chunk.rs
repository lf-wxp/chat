///
/// limitation of every chunk is 16k;
/// 
/// chunk type
/// 0 whole chunk
/// 1 splitted chunks
/// 2 destination mapping
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
/// including 8 bits for single char,  so the 22 chars need  176 bits(22 * 8)
/// 
/// 16k = 131072 bits
///
/// destination chunk
/// +------------+-------------+--------------+
/// | chunk type |  requestId  | mapping data |
/// +------------+-------------+--------------+
/// |   2 bits   |  176 bits   |  left bits   |
/// +------------+-------------+--------------+
/// 
/// data chunk
///                                          +-----------------------------+               +---------------------------+
///                                          |      for splitted chunks    |               |not for text type,max 500m |
/// +------------+-------------+-------------+--------------+--------------+--------------+-------------+-------------+-------------+
/// | chunk type |  requestId  | destination |   sequence   | total chunks | content type |  media name | media size  |   content   |
/// +------------+-------------+-------------+--------------+--------------+--------------+-------------+-------------+-------------+
/// |   2 bits   |  176 bits   |  176 bits   |   16 bits    |   16 bits    |    3 bits    |  400 bits   |   34 bits   |  left bits  |
/// +------------+-------------+-------------+--------------+--------------+--------------+-------------+-------------+-------------+
/// 
/// 
pub fn parser() {

}
