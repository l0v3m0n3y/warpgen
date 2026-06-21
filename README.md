# warpgen
mini -rust collection api for generate warp config
# main
```rust
mod warpgen;
use warpgen::{WarpGen};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let warp_gen = WarpGen::new();

    match warp_gen.save_warpgen_config().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }

    match warp_gen.save_valokda_config().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }
    
    match warp_gen.save_config_auto().await {
        Ok(path) => {
            println!("✅ Success! Config saved to: {}", path.display());
        }
        Err(e) => {
            eprintln!("❌ Error saving config: {}", e);
        }
    }

    let path = warp_gen.save_valokda_awg2_config().await?;
    println!("✅ Success! Config saved to: {}", path.display());
    
    Ok(())
}

```

# Launch (your script)
```
cargo run
```
