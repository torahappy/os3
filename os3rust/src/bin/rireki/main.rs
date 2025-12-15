// りれきしょ

use std::cell::RefCell;

fn main () -> Result<(), String> {
  thread_local!(
    pub static SOMEVAR: RefCell<Vec<u8>> = RefCell::new(Vec::new())
  );
  SOMEVAR.with(|x|{
      let mut m = x.borrow_mut();
      m.push(7);
  });
  Ok(())
}
