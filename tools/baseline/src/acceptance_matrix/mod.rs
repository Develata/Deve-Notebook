//! Machine-readable acceptance matrix and first-tag evidence gate.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use crate::context::BaselineContext;
use anyhow::{Result, bail};
use std::path::Path;

mod model;
mod parse;
mod receipt;
mod render;
mod tag_ready;
mod test_selector;
mod validate;

pub(crate) fn run(args: &[String]) -> Result<()> {
    let ctx = BaselineContext::new("acceptance-matrix")?;
    let rows = parse::read_matrix(ctx.root())?;
    validate::validate(ctx.root(), &rows)?;
    match args {
        [] => render::check_drift(ctx.root(), &rows)?,
        [action] if action == "--render" => render::write(ctx.root(), &rows)?,
        [action, receipt_dir] if action == "--tag-ready" => {
            let path = Path::new(receipt_dir);
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                ctx.root().join(path)
            };
            tag_ready::validate(ctx.root(), &rows, &path)?;
        }
        _ => {
            bail!("acceptance-matrix: expected no args, `--render`, or `--tag-ready <receipt-dir>`")
        }
    }
    println!("acceptance-matrix: {} requirement(s) ok", rows.len());
    Ok(())
}

pub(crate) fn run_receipt(args: &[String]) -> Result<()> {
    receipt::run(args)
}
