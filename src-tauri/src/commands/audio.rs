#[tauri::command]
pub fn play_audio(path: String, volume: f32) -> Result<(), String> {
    std::thread::spawn(move || {
        let Ok(file) = std::fs::File::open(&path) else {
            return;
        };
        let buf = std::io::BufReader::new(file);
        let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
            return;
        };
        let Ok(sink) = rodio::Sink::try_new(&handle) else {
            return;
        };
        let Ok(source) = rodio::Decoder::new(buf) else {
            return;
        };
        sink.set_volume(volume.clamp(0.0, 1.0));
        sink.append(source);
        sink.sleep_until_end();
    });

    Ok(())
}
