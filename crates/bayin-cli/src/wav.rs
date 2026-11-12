//! Sound to a file: raw f32 for `.f32`, otherwise a 16-bit mono WAV.
pub fn write(path: &str, audio: &[f32], fs: u32) -> Result<(), String> {
    let bytes: Vec<u8> = if path.ends_with(".f32") {
        audio.iter().flat_map(|x| x.to_le_bytes()).collect()
    } else {
        let data: Vec<u8> = audio.iter().flat_map(|x| ((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()).collect();
        let mut b = Vec::with_capacity(44 + data.len());
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&1u16.to_le_bytes()); // mono
        b.extend_from_slice(&fs.to_le_bytes());
        b.extend_from_slice(&(fs * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        b
    };
    std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))
}
