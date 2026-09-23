/// Photoshop PSD で使用される PackBits (ByteRun1) RLE デコーダー/エンコーダー
pub struct PackBits;

impl PackBits {
    /// PackBits 圧縮データのデコード
    pub fn decode(src: &[u8], expected_len: usize) -> Result<Vec<u8>, String> {
        let mut out = Vec::with_capacity(expected_len);
        let mut i = 0;

        while i < src.len() && out.len() < expected_len {
            let n = src[i] as i8;
            i += 1;

            if n >= 0 {
                // 0 から 127: 次の (n + 1) バイトをそのままコピー
                let count = (n as usize) + 1;
                if i + count > src.len() {
                    return Err("PackBits decode error: unexpected end of stream".into());
                }
                out.extend_from_slice(&src[i..i + count]);
                i += count;
            } else if n > -128 {
                // -1 から -127: 次の 1 バイトを (1 - n) 回繰り返す
                if i >= src.len() {
                    return Err("PackBits decode error: missing repeat byte".into());
                }
                let val = src[i];
                i += 1;
                let count = (1 - (n as i16)) as usize;
                out.resize(out.len() + count, val);
            }
            // n == -128 は No-op
        }

        if out.len() < expected_len {
            out.resize(expected_len, 0);
        } else {
            out.truncate(expected_len);
        }

        Ok(out)
    }

    /// バイト列を PackBits (RLE) 圧縮
    pub fn encode(src: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(src.len());
        let mut i = 0;

        while i < src.len() {
            // 連続する重複バイトの検出
            let mut run_len = 1;
            while i + run_len < src.len() && run_len < 128 && src[i + run_len] == src[i] {
                run_len += 1;
            }

            if run_len >= 3 {
                // RLE リピートチャンク
                let n = (1 - (run_len as i16)) as i8;
                out.push(n as u8);
                out.push(src[i]);
                i += run_len;
            } else {
                // リテラルチャンクの探索
                let mut lit_len = 0;
                while i + lit_len < src.len() && lit_len < 128 {
                    // 先頭から3バイト以上同一ならリピートに切り替え
                    if i + lit_len + 2 < src.len()
                        && src[i + lit_len] == src[i + lit_len + 1]
                        && src[i + lit_len] == src[i + lit_len + 2]
                    {
                        break;
                    }
                    lit_len += 1;
                }

                if lit_len > 0 {
                    let n = (lit_len - 1) as u8;
                    out.push(n);
                    out.extend_from_slice(&src[i..i + lit_len]);
                    i += lit_len;
                }
            }
        }

        out
    }
}
