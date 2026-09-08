use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

/// Packs raw bytes (e.g., a WAV file) into an RGBA pixel buffer (Uint8Array).
/// The first 4 bytes (first pixel) store the original byte length of the data as a 32-bit unsigned integer (big-endian).
/// The subsequent bytes are the original file bytes, padded with zeros to align to 4-byte boundaries.
#[wasm_bindgen]
pub fn encode_naive(data: &[u8]) -> Vec<u8> {
    let original_len = data.len() as u32;
    let mut encoded = Vec::with_capacity(4 + data.len() + 3); // space for len + data + potential padding
    
    // Store original length in the first 4 bytes (big-endian)
    encoded.extend_from_slice(&original_len.to_be_bytes());
    
    // Store the actual data
    encoded.extend_from_slice(data);
    
    // Pad with zeros to align to 4-byte (pixel) boundary
    let remainder = encoded.len() % 4;
    if remainder != 0 {
        let padding_needed = 4 - remainder;
        encoded.resize(encoded.len() + padding_needed, 0);
    }
    
    encoded
}

/// Unpacks raw bytes from an RGBA pixel buffer back into the original file bytes.
/// Reads the length from the first 4 bytes and extracts exactly that many bytes.
#[wasm_bindgen]
pub fn decode_naive(rgba_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    if rgba_data.len() < 4 {
        return Err(JsValue::from_str("Invalid encoded data: too short to contain length prefix."));
    }
    
    // Read original length from first 4 bytes
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&rgba_data[0..4]);
    let original_len = u32::from_be_bytes(len_bytes) as usize;
    
    if rgba_data.len() < 4 + original_len {
        return Err(JsValue::from_str(&format!(
            "Invalid encoded data: expected at least {} bytes (4 + {}), but got only {} bytes.",
            4 + original_len, original_len, rgba_data.len()
        )));
    }
    
    // Extract original bytes
    let original_data = rgba_data[4..4 + original_len].to_vec();
    Ok(original_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symmetrical_identity_naive() {
        // Create dummy "audio" bytes
        let original_audio = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22];
        
        // Encode
        let encoded_image = encode_naive(&original_audio);
        
        // Verify length and alignment
        assert_eq!(encoded_image.len() % 4, 0);
        assert!(encoded_image.len() >= 4 + original_audio.len());
        
        // Decode
        let decoded_audio = decode_naive(&encoded_image).unwrap();
        
        // Verify identical bytes
        assert_eq!(original_audio, decoded_audio);
    }
    
    #[test]
    fn test_symmetrical_empty() {
        let original_audio: Vec<u8> = vec![];
        let encoded_image = encode_naive(&original_audio);
        assert_eq!(encoded_image.len(), 4); // Just length prefix (0)
        let decoded_audio = decode_naive(&encoded_image).unwrap();
        assert_eq!(original_audio, decoded_audio);
    }
}
