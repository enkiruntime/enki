pub struct ImageCopyParams {
    pub row_pitch: usize,
    pub slice_pitch: usize,
    pub total_size_bytes: usize,
}

#[inline]
pub fn align_up(value: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return value;
    }
    (value + alignment - 1) & !(alignment - 1)
}

pub fn calculate_image_copy_params(
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    row_pitch_alignment: usize,
) -> ImageCopyParams {
    let row_bytes = width as usize * bytes_per_pixel as usize;
    let aligned_row_pitch = align_up(row_bytes, row_pitch_alignment);
    let slice_pitch = aligned_row_pitch * height as usize;

    ImageCopyParams {
        row_pitch: aligned_row_pitch,
        slice_pitch,
        total_size_bytes: slice_pitch,
    }
}

pub fn pad_pixel_data(
    src: &[u8],
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    row_pitch_alignment: usize,
) -> Vec<u8> {
    let params = calculate_image_copy_params(width, height, bytes_per_pixel, row_pitch_alignment);
    let src_row_bytes = width as usize * bytes_per_pixel as usize;

    if params.row_pitch == src_row_bytes {
        return src.to_vec();
    }

    let mut dst = vec![0u8; params.total_size_bytes];
    for row in 0..height as usize {
        let src_offset = row * src_row_bytes;
        let dst_offset = row * params.row_pitch;
        dst[dst_offset..dst_offset + src_row_bytes]
            .copy_from_slice(&src[src_offset..src_offset + src_row_bytes]);
    }
    dst
}

pub fn depad_pixel_data(
    src: &[u8],
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    row_pitch_alignment: usize,
) -> Vec<u8> {
    let params = calculate_image_copy_params(width, height, bytes_per_pixel, row_pitch_alignment);
    let dst_row_bytes = width as usize * bytes_per_pixel as usize;

    if params.row_pitch == dst_row_bytes {
        return src.to_vec();
    }

    let mut dst = vec![0u8; dst_row_bytes * height as usize];
    for row in 0..height as usize {
        let src_offset = row * params.row_pitch;
        let dst_offset = row * dst_row_bytes;
        dst[dst_offset..dst_offset + dst_row_bytes]
            .copy_from_slice(&src[src_offset..src_offset + dst_row_bytes]);
    }
    dst
}