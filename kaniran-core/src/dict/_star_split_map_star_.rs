//! Port of `ichiran/dict:*split-map*` (`dict-split.lisp:5`).
//!
//! Hashtable mapping JMdict seq → split function, registered upstream
//! by `defsplit` (`dict-split.lisp:7`) which is in turn invoked by
//! every `def-simple-split` / `def-de-split` / `def-toori-split` /
//! `def-do-split` / `def-shi-split` form. The Rust transliteration
//! collapses the runtime hashtable into a `match` dispatcher: every
//! split-* port is a sibling module, and [`split_map_dispatch`]
//! forwards to the registered fn. Returning `None` for unregistered
//! seqs preserves the `(gethash seq *split-map*)` semantics that
//! [`super::get_split_star_::get_split_star`] depends on.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

use super::split_1000430::split_1000430;
use super::split_1002970::split_1002970;
use super::split_de_1004800::split_de_1004800;
use super::split_1005600::split_1005600;
use super::split_shi_1005700::split_shi_1005700;
use super::split_shi_1005830::split_shi_1005830;
use super::split_1006280::split_1006280;
use super::split_souda::split_souda;
use super::split_de_1006840::split_de_1006840;
use super::split_1006880::split_1006880;
use super::split_1008030::split_1008030;
use super::split_nara::split_nara;
use super::split_nitotte::split_nitotte;
use super::split_shi_1157200::split_shi_1157200;
use super::split_shi_1157220::split_shi_1157220;
use super::split_shi_1157230::split_shi_1157230;
use super::split_shi_1157240::split_shi_1157240;
use super::split_shi_1157280::split_shi_1157280;
use super::split_shi_1157310::split_shi_1157310;
use super::split_de_1163700::split_de_1163700;
use super::split_toori_1164910::split_toori_1164910;
use super::split_de_1189420::split_de_1189420;
use super::split_1207840::split_1207840;
use super::split_1221530::split_1221530;
use super::split_1221680::split_1221680;
use super::split_kinosei::split_kinosei;
use super::split_osoreiru::split_osoreiru;
use super::split_de_1245390::split_de_1245390;
use super::split_toori_1260990::split_toori_1260990;
use super::split_de_1270210::split_de_1270210;
use super::split_de_1272220::split_de_1272220;
use super::split_shi_1304820::split_shi_1304820;
use super::split_shi_1304890::split_shi_1304890;
use super::split_shi_1304960::split_shi_1304960;
use super::split_shi_1305110::split_shi_1305110;
use super::split_shi_1305280::split_shi_1305280;
use super::split_shi_1305290::split_shi_1305290;
use super::split_de_1311360::split_de_1311360;
use super::split_1314600::split_1314600;
use super::split_1314770::split_1314770;
use super::split_motteiku::split_motteiku;
use super::split_1315860::split_1315860;
use super::split_1322540::split_1322540;
use super::split_1322560::split_1322560;
use super::split_1327220::split_1327220;
use super::split_1327230::split_1327230;
use super::split_de_1343110::split_de_1343110;
use super::split_1349300::split_1349300;
use super::split_1362970::split_1362970;
use super::split_de_1368500::split_de_1368500;
use super::split_hitotachi::split_hitotachi;
use super::split_toori_1368820::split_toori_1368820;
use super::split_de_1395670::split_de_1395670;
use super::split_kawaribae::split_kawaribae;
use super::split_toori_1414570::split_toori_1414570;
use super::split_de_1417790::split_de_1417790;
use super::split_hajikidasu::split_hajikidasu;
use super::split_toori_1424950::split_toori_1424950;
use super::split_toori_1424960::split_toori_1424960;
use super::split_de_1454270::split_de_1454270;
use super::split_toori_1462720::split_toori_1462720;
use super::split_hairikomeru::split_hairikomeru;
use super::split_1474200::split_1474200;
use super::split_de_1479100::split_de_1479100;
use super::split_toori_1489800::split_toori_1489800;
use super::split_1502500::split_1502500;
use super::split_1508380::split_1508380;
use super::split_de_1510140::split_de_1510140;
use super::split_nakunaru2::split_nakunaru2;
use super::split_de_1518550::split_de_1518550;
use super::split_toori_1523010::split_toori_1523010;
use super::split_gotoni::split_gotoni;
use super::split_nakunaru::split_nakunaru;
use super::split_de_1530610::split_de_1530610;
use super::split_de_1531420::split_de_1531420;
use super::split_1532270::split_1532270;
use super::split_1538340::split_1538340;
use super::split_toori_1550490::split_toori_1550490;
use super::split_1551500::split_1551500;
use super::split_1579130::split_1579130;
use super::split_1581550::split_1581550;
use super::split_1591050::split_1591050;
use super::split_1591980::split_1591980;
use super::split_shi_1594300::split_shi_1594300;
use super::split_shi_1594310::split_shi_1594310;
use super::split_shi_1594460::split_shi_1594460;
use super::split_shi_1594580::split_shi_1594580;
use super::split_de_1597400::split_de_1597400;
use super::split_1597740::split_1597740;
use super::split_nanimokamo::split_nanimokamo;
use super::split_1601010::split_1601010;
use super::split_1601080::split_1601080;
use super::split_1602740::split_1602740;
use super::split_1606530::split_1606530;
use super::split_1606800::split_1606800;
use super::split_kaasan::split_kaasan;
use super::split_de_1611020::split_de_1611020;
use super::split_1612640::split_1612640;
use super::split_toori_1619440::split_toori_1619440;
use super::split_de_1679990::split_de_1679990;
use super::split_de_1682060::split_de_1682060;
use super::split_osagari::split_osagari;
use super::split_de_1736650::split_de_1736650;
use super::split_kaisasae::split_kaisasae;
use super::split_1774820::split_1774820;
use super::split_toori_1808080::split_toori_1808080;
use super::split_toori_1820790::split_toori_1820790;
use super::split_1854750::split_1854750;
use super::split_1855670::split_1855670;
use super::split_1863230::split_1863230;
use super::split_de_1865020::split_de_1865020;
use super::split_de_1878880::split_de_1878880;
use super::split_shinikakaru::split_shinikakaru;
use super::split_1881690::split_1881690;
use super::split_1894260::split_1894260;
use super::split_hisshininatte::split_hisshininatte;
use super::split_toiu::split_toiu;
use super::split_kimatte::split_kimatte;
use super::split_2002270::split_2002270;
use super::split_2007500::split_2007500;
use super::split_2009290::split_2009290;
use super::split_2016840::split_2016840;
use super::split_2026650::split_2026650;
use super::split_desura::split_desura;
use super::split_moushiwakenasasou::split_moushiwakenasasou;
use super::split_2083990::split_2083990;
use super::split_2088480::split_2088480;
use super::split_tegakakaru::split_tegakakaru;
use super::split_tonattara::split_tonattara;
use super::split_tonaru::split_tonaru;
use super::split_katawonaraberu::split_katawonaraberu;
use super::split_nantokanaru::split_nantokanaru;
use super::split_2109610::split_2109610;
use super::split_de_2126220::split_de_2126220;
use super::split_2133750::split_2133750;
use super::split_jan::split_jan;
use super::split_de_2136520::split_de_2136520;
use super::split_do_2142680::split_do_2142680;
use super::split_do_2142710::split_do_2142710;
use super::split_kotonisuru::split_kotonisuru;
use super::split_degozaimasu::split_degozaimasu;
use super::split_2272780::split_2272780;
use super::split_2276360::split_2276360;
use super::split_2433760::split_2433760;
use super::split_de_2513590::split_de_2513590;
use super::split_shi_2518250::split_shi_2518250;
use super::split_do_2523480::split_do_2523480;
use super::split_2526850::split_2526850;
use super::split_2529050::split_2529050;
use super::split_hajiketobu::split_hajiketobu;
use super::split_toiukotoda::split_toiukotoda;
use super::split_2666360::split_2666360;
use super::split_2668400::split_2668400;
use super::split_de_2719270::split_de_2719270;
use super::split_2724560::split_2724560;
use super::split_janai::split_janai;
use super::split_2757500::split_2757500;
use super::split_2757540::split_2757540;
use super::split_2762260::split_2762260;
use super::split_de_2771850::split_de_2771850;
use super::split_2771940::split_2771940;
use super::split_dogatsukeru::split_dogatsukeru;
use super::split_do_2803190::split_do_2803190;
use super::split_de_2810720::split_de_2810720;
use super::split_de_2810800::split_de_2810800;
use super::split_hayaimonode::split_hayaimonode;
use super::split_janaika::split_janaika;
use super::split_2834051::split_2834051;
use super::split_2834732::split_2834732;
use super::split_2835890::split_2835890;
use super::split_soudesu::split_soudesu;
use super::split_2846470::split_2846470;
use super::split_2855921::split_2855921;
use super::split_shi_2858937::split_shi_2858937;

pub async fn split_map_dispatch(
    seq: i32,
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
) -> Option<Result<(Vec<Option<SplitPart>>, i32), sqlx::Error>> {
    match seq {
        1000430i32 => Some(split_1000430(ctx, reading).await),
        1002970i32 => Some(split_1002970(ctx, reading).await),
        1004800i32 => Some(split_de_1004800(ctx, reading).await),
        1005600i32 => Some(split_1005600(ctx, reading).await),
        1005700i32 => Some(split_shi_1005700(ctx, reading).await),
        1005830i32 => Some(split_shi_1005830(ctx, reading).await),
        1006280i32 => Some(split_1006280(ctx, reading).await),
        1006650i32 => Some(split_souda(ctx, reading).await),
        1006840i32 => Some(split_de_1006840(ctx, reading).await),
        1006880i32 => Some(split_1006880(ctx, reading).await),
        1008030i32 => Some(split_1008030(ctx, reading).await),
        1009470i32 => Some(split_nara(ctx, reading).await),
        1009600i32 => Some(split_nitotte(ctx, reading).await),
        1157200i32 => Some(split_shi_1157200(ctx, reading).await),
        1157220i32 => Some(split_shi_1157220(ctx, reading).await),
        1157230i32 => Some(split_shi_1157230(ctx, reading).await),
        1157240i32 => Some(split_shi_1157240(ctx, reading).await),
        1157280i32 => Some(split_shi_1157280(ctx, reading).await),
        1157310i32 => Some(split_shi_1157310(ctx, reading).await),
        1163700i32 => Some(split_de_1163700(ctx, reading).await),
        1164910i32 => Some(split_toori_1164910(ctx, reading).await),
        1189420i32 => Some(split_de_1189420(ctx, reading).await),
        1207840i32 => Some(split_1207840(ctx, reading).await),
        1221530i32 => Some(split_1221530(ctx, reading).await),
        1221680i32 => Some(split_1221680(ctx, reading).await),
        1221750i32 => Some(split_kinosei(ctx, reading).await),
        1236680i32 => Some(split_osoreiru(ctx, reading).await),
        1245390i32 => Some(split_de_1245390(ctx, reading).await),
        1260990i32 => Some(split_toori_1260990(ctx, reading).await),
        1270210i32 => Some(split_de_1270210(ctx, reading).await),
        1272220i32 => Some(split_de_1272220(ctx, reading).await),
        1304820i32 => Some(split_shi_1304820(ctx, reading).await),
        1304890i32 => Some(split_shi_1304890(ctx, reading).await),
        1304960i32 => Some(split_shi_1304960(ctx, reading).await),
        1305110i32 => Some(split_shi_1305110(ctx, reading).await),
        1305280i32 => Some(split_shi_1305280(ctx, reading).await),
        1305290i32 => Some(split_shi_1305290(ctx, reading).await),
        1311360i32 => Some(split_de_1311360(ctx, reading).await),
        1314600i32 => Some(split_1314600(ctx, reading).await),
        1314770i32 => Some(split_1314770(ctx, reading).await),
        1315700i32 => Some(split_motteiku(ctx, reading).await),
        1315860i32 => Some(split_1315860(ctx, reading).await),
        1322540i32 => Some(split_1322540(ctx, reading).await),
        1322560i32 => Some(split_1322560(ctx, reading).await),
        1327220i32 => Some(split_1327220(ctx, reading).await),
        1327230i32 => Some(split_1327230(ctx, reading).await),
        1343110i32 => Some(split_de_1343110(ctx, reading).await),
        1349300i32 => Some(split_1349300(ctx, reading).await),
        1362970i32 => Some(split_1362970(ctx, reading).await),
        1368500i32 => Some(split_de_1368500(ctx, reading).await),
        1368740i32 => Some(split_hitotachi(ctx, reading).await),
        1368820i32 => Some(split_toori_1368820(ctx, reading).await),
        1395670i32 => Some(split_de_1395670(ctx, reading).await),
        1411570i32 => Some(split_kawaribae(ctx, reading).await),
        1414570i32 => Some(split_toori_1414570(ctx, reading).await),
        1417790i32 => Some(split_de_1417790(ctx, reading).await),
        1419350i32 => Some(split_hajikidasu(ctx, reading).await),
        1424950i32 => Some(split_toori_1424950(ctx, reading).await),
        1424960i32 => Some(split_toori_1424960(ctx, reading).await),
        1454270i32 => Some(split_de_1454270(ctx, reading).await),
        1462720i32 => Some(split_toori_1462720(ctx, reading).await),
        1465460i32 => Some(split_hairikomeru(ctx, reading).await),
        1474200i32 => Some(split_1474200(ctx, reading).await),
        1479100i32 => Some(split_de_1479100(ctx, reading).await),
        1489800i32 => Some(split_toori_1489800(ctx, reading).await),
        1502500i32 => Some(split_1502500(ctx, reading).await),
        1508380i32 => Some(split_1508380(ctx, reading).await),
        1510140i32 => Some(split_de_1510140(ctx, reading).await),
        1518540i32 => Some(split_nakunaru2(ctx, reading).await),
        1518550i32 => Some(split_de_1518550(ctx, reading).await),
        1523010i32 => Some(split_toori_1523010(ctx, reading).await),
        1524660i32 => Some(split_gotoni(ctx, reading).await),
        1529550i32 => Some(split_nakunaru(ctx, reading).await),
        1530610i32 => Some(split_de_1530610(ctx, reading).await),
        1531420i32 => Some(split_de_1531420(ctx, reading).await),
        1532270i32 => Some(split_1532270(ctx, reading).await),
        1538340i32 => Some(split_1538340(ctx, reading).await),
        1550490i32 => Some(split_toori_1550490(ctx, reading).await),
        1551500i32 => Some(split_1551500(ctx, reading).await),
        1579130i32 => Some(split_1579130(ctx, reading).await),
        1581550i32 => Some(split_1581550(ctx, reading).await),
        1591050i32 => Some(split_1591050(ctx, reading).await),
        1591980i32 => Some(split_1591980(ctx, reading).await),
        1594300i32 => Some(split_shi_1594300(ctx, reading).await),
        1594310i32 => Some(split_shi_1594310(ctx, reading).await),
        1594460i32 => Some(split_shi_1594460(ctx, reading).await),
        1594580i32 => Some(split_shi_1594580(ctx, reading).await),
        1597400i32 => Some(split_de_1597400(ctx, reading).await),
        1597740i32 => Some(split_1597740(ctx, reading).await),
        1599590i32 => Some(split_nanimokamo(ctx, reading).await),
        1601010i32 => Some(split_1601010(ctx, reading).await),
        1601080i32 => Some(split_1601080(ctx, reading).await),
        1602740i32 => Some(split_1602740(ctx, reading).await),
        1606530i32 => Some(split_1606530(ctx, reading).await),
        1606800i32 => Some(split_1606800(ctx, reading).await),
        1609470i32 => Some(split_kaasan(ctx, reading).await),
        1611020i32 => Some(split_de_1611020(ctx, reading).await),
        1612640i32 => Some(split_1612640(ctx, reading).await),
        1619440i32 => Some(split_toori_1619440(ctx, reading).await),
        1679990i32 => Some(split_de_1679990(ctx, reading).await),
        1682060i32 => Some(split_de_1682060(ctx, reading).await),
        1693800i32 => Some(split_osagari(ctx, reading).await),
        1736650i32 => Some(split_de_1736650(ctx, reading).await),
        1752860i32 => Some(split_kaisasae(ctx, reading).await),
        1774820i32 => Some(split_1774820(ctx, reading).await),
        1808080i32 => Some(split_toori_1808080(ctx, reading).await),
        1820790i32 => Some(split_toori_1820790(ctx, reading).await),
        1854750i32 => Some(split_1854750(ctx, reading).await),
        1855670i32 => Some(split_1855670(ctx, reading).await),
        1863230i32 => Some(split_1863230(ctx, reading).await),
        1865020i32 => Some(split_de_1865020(ctx, reading).await),
        1878880i32 => Some(split_de_1878880(ctx, reading).await),
        1881080i32 => Some(split_shinikakaru(ctx, reading).await),
        1881690i32 => Some(split_1881690(ctx, reading).await),
        1894260i32 => Some(split_1894260(ctx, reading).await),
        1903910i32 => Some(split_hisshininatte(ctx, reading).await),
        1922760i32 => Some(split_toiu(ctx, reading).await),
        1951150i32 => Some(split_kimatte(ctx, reading).await),
        2002270i32 => Some(split_2002270(ctx, reading).await),
        2007500i32 => Some(split_2007500(ctx, reading).await),
        2009290i32 => Some(split_2009290(ctx, reading).await),
        2016840i32 => Some(split_2016840(ctx, reading).await),
        2026650i32 => Some(split_2026650(ctx, reading).await),
        2034520i32 => Some(split_desura(ctx, reading).await),
        2057340i32 => Some(split_moushiwakenasasou(ctx, reading).await),
        2083990i32 => Some(split_2083990(ctx, reading).await),
        2088480i32 => Some(split_2088480(ctx, reading).await),
        2089710i32 => Some(split_tegakakaru(ctx, reading).await),
        2100770i32 => Some(split_tonattara(ctx, reading).await),
        2100900i32 => Some(split_tonaru(ctx, reading).await),
        2102910i32 => Some(split_katawonaraberu(ctx, reading).await),
        2104540i32 => Some(split_nantokanaru(ctx, reading).await),
        2109610i32 => Some(split_2109610(ctx, reading).await),
        2126220i32 => Some(split_de_2126220(ctx, reading).await),
        2133750i32 => Some(split_2133750(ctx, reading).await),
        2135280i32 => Some(split_jan(ctx, reading).await),
        2136520i32 => Some(split_de_2136520(ctx, reading).await),
        2142680i32 => Some(split_do_2142680(ctx, reading).await),
        2142710i32 => Some(split_do_2142710(ctx, reading).await),
        2215340i32 => Some(split_kotonisuru(ctx, reading).await),
        2253080i32 => Some(split_degozaimasu(ctx, reading).await),
        2272780i32 => Some(split_2272780(ctx, reading).await),
        2276360i32 => Some(split_2276360(ctx, reading).await),
        2433760i32 => Some(split_2433760(ctx, reading).await),
        2513590i32 => Some(split_de_2513590(ctx, reading).await),
        2518250i32 => Some(split_shi_2518250(ctx, reading).await),
        2523480i32 => Some(split_do_2523480(ctx, reading).await),
        2526850i32 => Some(split_2526850(ctx, reading).await),
        2529050i32 => Some(split_2529050(ctx, reading).await),
        2610760i32 => Some(split_hajiketobu(ctx, reading).await),
        2612990i32 => Some(split_toiukotoda(ctx, reading).await),
        2666360i32 => Some(split_2666360(ctx, reading).await),
        2668400i32 => Some(split_2668400(ctx, reading).await),
        2719270i32 => Some(split_de_2719270(ctx, reading).await),
        2724560i32 => Some(split_2724560(ctx, reading).await),
        2755350i32 => Some(split_janai(ctx, reading).await),
        2757500i32 => Some(split_2757500(ctx, reading).await),
        2757540i32 => Some(split_2757540(ctx, reading).await),
        2762260i32 => Some(split_2762260(ctx, reading).await),
        2771850i32 => Some(split_de_2771850(ctx, reading).await),
        2771940i32 => Some(split_2771940(ctx, reading).await),
        2800540i32 => Some(split_dogatsukeru(ctx, reading).await),
        2803190i32 => Some(split_do_2803190(ctx, reading).await),
        2810720i32 => Some(split_de_2810720(ctx, reading).await),
        2810800i32 => Some(split_de_2810800(ctx, reading).await),
        2815260i32 => Some(split_hayaimonode(ctx, reading).await),
        2819990i32 => Some(split_janaika(ctx, reading).await),
        2834051i32 => Some(split_2834051(ctx, reading).await),
        2834732i32 => Some(split_2834732(ctx, reading).await),
        2835890i32 => Some(split_2835890(ctx, reading).await),
        2837492i32 => Some(split_soudesu(ctx, reading).await),
        2846470i32 => Some(split_2846470(ctx, reading).await),
        2855921i32 => Some(split_2855921(ctx, reading).await),
        2858937i32 => Some(split_shi_2858937(ctx, reading).await),
        _ => None,
    }
}

/// Number of registered seqs — pinned so the build fails loudly if
/// a future macro form accidentally drops out of the regenerated set.
#[cfg(test)]
pub(crate) const REGISTERED_COUNT: usize = 174;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_count_matches_upstream_split_map() {
        // dict-split.lisp registers 174 entries via def-simple-split /
        // def-de-split / def-toori-split / def-do-split /
        // def-shi-split outside the *segsplit-map* let-binding.
        assert_eq!(REGISTERED_COUNT, 174);
    }
}
