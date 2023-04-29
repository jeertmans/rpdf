use anyhow::Result;
use termcolor::WriteColor;

pub trait Execute {
    fn execute<W: WriteColor>(&self, stdout: &mut W) -> Result<()>;
}
