# warpgen
mini -rust collection api for generate warp config
# main
```rust
mod warpgen;
use warpgen::WarpGen;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let warp_client = WarpGen::new();

    match warp_client.save_warpgen_config().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }

    match warp_client.save_portal_config().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }

    match warp_client.save_warp_workers_config("de_DE").await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }

    match warp_client.save_valokda_config().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }
    
    match warp_client.save_config_auto().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }

    Ok(())
}

```

# Launch (your script)
```
cargo run
```
