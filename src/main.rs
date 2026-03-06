mod app;
mod cli_output;
mod dp_info;
mod drm_info;
mod edid;
mod gpu;
mod render;

use clap::Parser;

#[derive(Parser)]
#[command(name = "display-analyzer", about = "Linux display diagnostic tool")]
struct Cli {
    /// Launch GUI mode
    #[arg(long)]
    gui: bool,

    /// Show only a specific port (e.g. card1-DP-4, card1-eDP-1)
    #[arg(long)]
    port: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    if cli.gui {
        run_gui();
    } else {
        let mut connectors = drm_info::enumerate_connectors();
        if let Some(port) = &cli.port {
            connectors.retain(|c| c.name == *port);
            if connectors.is_empty() {
                eprintln!("No connector found matching '{port}'");
                eprintln!(
                    "Available: {}",
                    drm_info::enumerate_connectors()
                        .iter()
                        .map(|c| c.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                std::process::exit(1);
            }
        }
        cli_output::print_all(&connectors);
    }
}

fn run_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Display Analyzer"),
        ..Default::default()
    };

    eframe::run_native(
        "Display Analyzer",
        options,
        Box::new(|_cc| Ok(Box::new(app::DisplayAnalyzerApp::new()))),
    )
    .expect("Failed to run eframe");
}
