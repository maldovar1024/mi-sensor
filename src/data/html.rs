use anyhow::Result;
use std::io::{self, BufWriter, Write};

use super::{get_temp, DataItem, Summary, Year};

trait WriteHtmlString {
    fn write_html_string(&self, w: &mut impl Write) -> io::Result<()>;
}

impl WriteHtmlString for DataItem {
    fn write_html_string(&self, w: &mut impl Write) -> io::Result<()> {
        write!(
            w,
            "<div>{},{}℃,{}℃,{}%,{}%</div>",
            self.time,
            get_temp(self.max_temperature),
            get_temp(self.min_temperature),
            self.max_humidity,
            self.min_humidity
        )
    }
}

impl<T: WriteHtmlString> WriteHtmlString for Summary<T> {
    fn write_html_string(&self, w: &mut impl Write) -> io::Result<()> {
        write!(w, "<details><summary>",)?;
        self.summary.write_html_string(w)?;
        write!(w, "</summary>",)?;
        for detail in self.details.iter() {
            detail.write_html_string(w)?;
        }
        write!(w, "</details>")
    }
}

pub fn gen_html_string<W: std::io::Write>(input: &[Year], mut output: BufWriter<W>) -> Result<()> {
    output.write_all(r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Document</title><style>details{margin-left: 20px;}summary>div{display: contents;}body{font-size: 14px;font-family: monospace;}</style></head><body>"#.as_bytes())?;

    for summary in input.iter() {
        summary.write_html_string(&mut output)?;
    }

    output.write_all("</body></html>".as_bytes())?;
    output.flush()?;

    Ok(())
}
