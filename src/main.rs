mod data;
mod sensor;
use std::{
    fs::{self, File},
    io::BufWriter,
};

use anyhow::Result;
use data::{get_summary, html::gen_html_string};
use sensor::update_data;

#[tokio::main]
async fn main() -> Result<()> {
    // 连接设备，更新数据
    update_data("../temp1.mi").await?;

    // 读取数据，生成 HTML
    let data = fs::read("../temp1.mi")?;
    let summaries = get_summary(&data);

    let output = BufWriter::new(
        File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open("index.html")?,
    );

    gen_html_string(&summaries, output)
}
