#[cfg(feature = "pipes")]
#[cfg(test)]
mod tests {
    use screenpipe_core::{download_pipe, run_js, run_pipe};
    use serde_json::json;
    use std::{path::PathBuf, sync::Once};
    use tempfile::TempDir;
    use tokio::{
        fs::{create_dir_all, File},
        io::AsyncWriteExt,
    };
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::fmt::Subscriber;

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            let subscriber = Subscriber::builder()
                .with_env_filter("debug")
                .with_test_writer()
                .finish();
            set_global_default(subscriber).expect("Failed to set tracing subscriber");
        });
    }
    #[tokio::test]
    async fn test_js_execution() {
        init();
        let code = r#"
            function add(a, b) {
                return a + b;
            }
            add(2, 3);
            console.log("Hello, world!");
            const response = await pipe.get("https://jsonplaceholder.typicode.com/todos/1");
            console.log(response);
            "#;

        // write code to a file
        let file_path = "test.js";
        let mut file = File::create(file_path).await.unwrap();
        file.write_all(code.as_bytes()).await.unwrap();
        file.flush().await.unwrap();
        // Test a simple JavaScript function
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let result = run_js("", file_path, screenpipe_dir).await;

        assert!(result.is_ok());
        println!("result: {:?}", result);
        // remove_file(file_path).await.unwrap();
    }

    async fn setup_test_pipe(temp_dir: &TempDir, pipe_name: &str, code: &str) -> PathBuf {
        init();
        let pipe_dir = temp_dir.path().join(pipe_name);
        create_dir_all(&pipe_dir).await.unwrap();
        let file_path = pipe_dir.join("pipe.ts");
        tokio::fs::write(&file_path, code).await.unwrap();
        pipe_dir
    }

    #[tokio::test]
    async fn test_simple_pipe() {
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let code = r#"
            console.log("Hello from simple pipe!");
            const result = 2 + 3;
            console.log(`Result: ${result}`);
        "#;

        let pipe_dir = setup_test_pipe(&temp_dir, "simple_pipe", code).await;

        let result = run_pipe(pipe_dir.to_string_lossy().to_string(), screenpipe_dir).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pipe_with_http_request() {
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let code = r#"
            console.log("Fetching data from API...");
            const response = await pipe.get("https://jsonplaceholder.typicode.com/todos/1");
            console.log(JSON.stringify(response, null, 2));
        "#;

        let pipe_dir = setup_test_pipe(&temp_dir, "http_pipe", code).await;

        let result = run_pipe(pipe_dir.to_string_lossy().to_string(), screenpipe_dir).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // TODO: fix this test (not implemented yet)
    async fn test_pipe_with_error() {
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let code = r#"
            console.log("This pipe will throw an error");
            throw new Error("Intentional error");
        "#;

        let pipe_dir = setup_test_pipe(&temp_dir, "error_pipe", code).await;

        let result = run_pipe(pipe_dir.to_string_lossy().to_string(), screenpipe_dir).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // TODO: fix this test (file operations work but not in this test for some reason)
    async fn test_pipe_with_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let code = r#"
            console.log("Writing to a file...");
            await pipe.writeFile("output.txt", "Hello, Screenpipe!");
            const content = await pipe.readFile("output.txt");
            console.log(`File content: ${content}`);
        "#;

        let pipe_dir = setup_test_pipe(&temp_dir, "file_pipe", code).await;

        let result = run_pipe(pipe_dir.to_string_lossy().to_string(), screenpipe_dir).await;
        assert!(result.is_ok());

        // Verify that the file was created and contains the expected content
        let output_file = pipe_dir.join("output.txt");
        assert!(output_file.exists());
        let content = tokio::fs::read_to_string(output_file).await.unwrap();
        assert_eq!(content, "Hello, Screenpipe!");
    }

    async fn setup_test_pipe_with_config(
        temp_dir: &TempDir,
        pipe_name: &str,
        code: &str,
        config: &str,
    ) -> PathBuf {
        init();
        let pipe_dir = temp_dir.path().join(pipe_name);
        create_dir_all(&pipe_dir).await.unwrap();

        let ts_file_path = pipe_dir.join("pipe.ts");
        tokio::fs::write(&ts_file_path, code).await.unwrap();

        let json_file_path = pipe_dir.join("pipe.json");
        tokio::fs::write(&json_file_path, config).await.unwrap();

        pipe_dir
    }

    #[tokio::test]
    async fn test_pipe_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let code = r#"
            (async () => {
                await pipe.loadConfig();
                console.log("Pipe config test");
                console.log(`API Key: ${pipe.config.apiKey}`);
                console.log(`Endpoint: ${pipe.config.endpoint}`);
                if (pipe.config.apiKey !== "test-api-key" || pipe.config.endpoint !== "https://api.example.com") {
                    throw new Error("Config not loaded correctly");
                }
            })();
        "#;

        let config = json!({
            "apiKey": "test-api-key",
            "endpoint": "https://api.example.com"
        })
        .to_string();

        let pipe_dir = setup_test_pipe_with_config(&temp_dir, "config_pipe", code, &config).await;

        // Change the working directory to the pipe directory
        std::env::set_current_dir(&pipe_dir).unwrap();

        let result = run_pipe(pipe_dir.to_string_lossy().to_string(), screenpipe_dir).await;
        assert!(result.is_ok(), "Pipe execution failed: {:?}", result);
    }

    #[tokio::test]
    #[ignore] // Github said NO
    async fn test_download_pipe_github_folder() {
        init();
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let github_url = "https://github.com/mediar-ai/screenpipe/tree/main/examples/typescript/pipe-stream-ocr-text";
        let result = download_pipe(github_url, screenpipe_dir.clone()).await;

        assert!(
            result.is_ok(),
            "Failed to download GitHub folder: {:?}",
            result
        );
        let pipe_dir = result.unwrap();
        assert!(pipe_dir.exists(), "Pipe directory does not exist");

        let has_main_or_pipe_file = std::fs::read_dir(&pipe_dir).unwrap().any(|entry| {
            let file_name = entry.unwrap().file_name().into_string().unwrap();
            (file_name.starts_with("main") || file_name.starts_with("pipe"))
                && (file_name.ends_with(".ts") || file_name.ends_with(".js"))
        });

        assert!(
            has_main_or_pipe_file,
            "No main.ts, main.js, pipe.ts, or pipe.js file found"
        );
    }

    #[tokio::test]
    async fn test_download_pipe_raw_file() {
        init();
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let raw_url = "https://raw.githubusercontent.com/mediar-ai/screenpipe/main/examples/typescript/pipe-stream-ocr-text/main.js";
        let result = download_pipe(raw_url, screenpipe_dir.clone()).await;

        assert!(result.is_ok(), "Failed to download raw file: {:?}", result);
        let pipe_dir = result.unwrap();
        assert!(pipe_dir.exists(), "Pipe directory does not exist");

        let has_main_or_pipe_file = std::fs::read_dir(&pipe_dir).unwrap().any(|entry| {
            let file_name = entry.unwrap().file_name().into_string().unwrap();
            (file_name.starts_with("main") || file_name.starts_with("pipe"))
                && (file_name.ends_with(".ts") || file_name.ends_with(".js"))
        });

        assert!(
            has_main_or_pipe_file,
            "No main.ts, main.js, pipe.ts, or pipe.js file found"
        );
    }

    #[tokio::test]
    async fn test_download_pipe_invalid_url() {
        init();
        let temp_dir = TempDir::new().unwrap();
        let screenpipe_dir = temp_dir.path().to_path_buf();

        let invalid_url = "https://example.com/invalid/url";
        let result = download_pipe(invalid_url, screenpipe_dir.clone()).await;

        assert!(result.is_err(), "Expected an error for invalid URL");
    }
}