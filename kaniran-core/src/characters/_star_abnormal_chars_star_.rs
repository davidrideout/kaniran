//! Port of `ichiran/characters:*abnormal-chars*`
//! (`characters.lisp:109`).
//!
//! Source side of the abnormal→normal character map: full-width ASCII
//! printables and half-width katakana, paired index-by-index with
//! `*normal-chars*`.

pub static ABNORMAL_CHARS: &str = "\
０１２３４５６７８９\
ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ\
ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
＃＄％＆（）＊＋／〈＝〉？＠［］＾＿‘｛｜｝～\
･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";
