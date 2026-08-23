from pathlib import Path

path = Path("varcomp_kss/varcomp_kss.ado")
text = path.read_text(encoding="utf-8")
old = """        leverage_batch_mode_code:r_lev_batch_mode                    ///
        target_batch_mode_code:r_tgt_batch_mode plan_schema:r_plan_schema ///
"""
new = """        batch_lev_mode:r_lev_batch_mode                              ///
        batch_tgt_mode:r_tgt_batch_mode plan_schema:r_plan_schema     ///
"""
count = text.count(old)
if count != 1:
    raise RuntimeError(f"V7 batch-mode aliases: expected one block, found {count}")
text = text.replace(old, new, 1)
path.write_text(text, encoding="utf-8")
print("repaired V7 batch-mode result names")
