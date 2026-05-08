use std::path::Path;

pub fn generate_sitemap(reports_dir: &Path) -> String {
    let mut sitemap = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    sitemap.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    if let Ok(entries) = std::fs::read_dir(reports_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(crate_name) = path.file_name().and_then(|n| n.to_str()) {
                    sitemap.push_str(&format!(
                        "<url><loc>https://edgerun.ai/reports/crates/{}/report.html</loc></url>\n",
                        crate_name
                    ));
                }
            }
        }
    }

    sitemap.push_str("</urlset>\n");
    sitemap
}
