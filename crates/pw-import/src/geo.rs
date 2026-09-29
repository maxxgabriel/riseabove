//! Country identity: canonical English names, FIFA-style codes, confederations, spelling aliases and the
//! French names used by the staff list. These are public facts about countries, kept in code so no adapter
//! has to guess: a name that is not here stays UNKNOWN.

use pw_world::nation::Confed::{self, Afc, Caf, Concacaf, Conmebol, Ofc, Uefa};

pub struct Geo {
    pub name: &'static str,
    pub code: &'static str,
    pub confed: Confed,
}

const fn g(name: &'static str, code: &'static str, confed: Confed) -> Geo {
    Geo { name, code, confed }
}

pub const COUNTRIES: &[Geo] = &[
    g("Afghanistan", "AFG", Afc),
    g("Albania", "ALB", Uefa),
    g("Algeria", "ALG", Caf),
    g("Andorra", "AND", Uefa),
    g("Angola", "ANG", Caf),
    g("Anguilla", "AIA", Concacaf),
    g("Antigua and Barbuda", "ATG", Concacaf),
    g("Argentina", "ARG", Conmebol),
    g("Armenia", "ARM", Uefa),
    g("Aruba", "ARU", Concacaf),
    g("Australia", "AUS", Afc),
    g("Austria", "AUT", Uefa),
    g("Azerbaijan", "AZE", Uefa),
    g("Bahamas", "BAH", Concacaf),
    g("Bahrain", "BHR", Afc),
    g("Bangladesh", "BAN", Afc),
    g("Barbados", "BRB", Concacaf),
    g("Belarus", "BLR", Uefa),
    g("Belgium", "BEL", Uefa),
    g("Belize", "BLZ", Concacaf),
    g("Benin", "BEN", Caf),
    g("Bermuda", "BER", Concacaf),
    g("Bhutan", "BHU", Afc),
    g("Bolivia", "BOL", Conmebol),
    g("Bonaire", "BON", Concacaf),
    g("Bosnia-Herzegovina", "BIH", Uefa),
    g("Botswana", "BOT", Caf),
    g("Brazil", "BRA", Conmebol),
    g("Brunei Darussalam", "BRU", Afc),
    g("Bulgaria", "BUL", Uefa),
    g("Burkina Faso", "BFA", Caf),
    g("Burundi", "BDI", Caf),
    g("Cambodia", "CAM", Afc),
    g("Cameroon", "CMR", Caf),
    g("Canada", "CAN", Concacaf),
    g("Cape Verde", "CPV", Caf),
    g("Central African Republic", "CTA", Caf),
    g("Chad", "CHA", Caf),
    g("Chile", "CHI", Conmebol),
    g("China", "CHN", Afc),
    g("Chinese Taipei", "TPE", Afc),
    g("Colombia", "COL", Conmebol),
    g("Comoros", "COM", Caf),
    g("Congo", "CGO", Caf),
    g("Costa Rica", "CRC", Concacaf),
    g("Cote d'Ivoire", "CIV", Caf),
    g("Croatia", "CRO", Uefa),
    g("Cuba", "CUB", Concacaf),
    g("Curacao", "CUW", Concacaf),
    g("Cyprus", "CYP", Uefa),
    g("Czech Republic", "CZE", Uefa),
    g("DR Congo", "COD", Caf),
    g("Denmark", "DEN", Uefa),
    g("Dominican Republic", "DOM", Concacaf),
    g("Ecuador", "ECU", Conmebol),
    g("Egypt", "EGY", Caf),
    g("El Salvador", "SLV", Concacaf),
    g("England", "ENG", Uefa),
    g("Equatorial Guinea", "EQG", Caf),
    g("Eritrea", "ERI", Caf),
    g("Estonia", "EST", Uefa),
    g("Ethiopia", "ETH", Caf),
    g("Faroe Islands", "FRO", Uefa),
    g("Fiji", "FIJ", Ofc),
    g("Finland", "FIN", Uefa),
    g("France", "FRA", Uefa),
    g("French Guiana", "GUF", Concacaf),
    g("Gabon", "GAB", Caf),
    g("Georgia", "GEO", Uefa),
    g("Germany", "GER", Uefa),
    g("Ghana", "GHA", Caf),
    g("Gibraltar", "GIB", Uefa),
    g("Greece", "GRE", Uefa),
    g("Grenada", "GRN", Concacaf),
    g("Guadeloupe", "GLP", Concacaf),
    g("Guatemala", "GUA", Concacaf),
    g("Guinea", "GUI", Caf),
    g("Guinea-Bissau", "GNB", Caf),
    g("Guyana", "GUY", Concacaf),
    g("Haiti", "HAI", Concacaf),
    g("Honduras", "HON", Concacaf),
    g("Hongkong", "HKG", Afc),
    g("Hungary", "HUN", Uefa),
    g("Iceland", "ISL", Uefa),
    g("India", "IND", Afc),
    g("Indonesia", "IDN", Afc),
    g("Iran", "IRN", Afc),
    g("Iraq", "IRQ", Afc),
    g("Ireland", "IRL", Uefa),
    g("Israel", "ISR", Uefa),
    g("Italy", "ITA", Uefa),
    g("Jamaica", "JAM", Concacaf),
    g("Japan", "JPN", Afc),
    g("Jordan", "JOR", Afc),
    g("Kazakhstan", "KAZ", Uefa),
    g("Kenya", "KEN", Caf),
    g("Korea, North", "PRK", Afc),
    g("Korea, South", "KOR", Afc),
    g("Kosovo", "KVX", Uefa),
    g("Kyrgyzstan", "KGZ", Afc),
    g("Laos", "LAO", Afc),
    g("Latvia", "LVA", Uefa),
    g("Lebanon", "LIB", Afc),
    g("Lesotho", "LES", Caf),
    g("Liberia", "LBR", Caf),
    g("Libya", "LBY", Caf),
    g("Liechtenstein", "LIE", Uefa),
    g("Lithuania", "LTU", Uefa),
    g("Luxembourg", "LUX", Uefa),
    g("Macao", "MAC", Afc),
    g("Madagascar", "MAD", Caf),
    g("Malawi", "MWI", Caf),
    g("Malaysia", "MAS", Afc),
    g("Maldives", "MDV", Afc),
    g("Mali", "MLI", Caf),
    g("Malta", "MLT", Uefa),
    g("Martinique", "MTQ", Concacaf),
    g("Mauritania", "MTN", Caf),
    g("Mauritius", "MRI", Caf),
    g("Mexico", "MEX", Concacaf),
    g("Moldova", "MDA", Uefa),
    g("Monaco", "MON", Uefa),
    g("Mongolia", "MNG", Afc),
    g("Montenegro", "MNE", Uefa),
    g("Montserrat", "MSR", Concacaf),
    g("Morocco", "MAR", Caf),
    g("Mozambique", "MOZ", Caf),
    g("Myanmar", "MYA", Afc),
    g("Namibia", "NAM", Caf),
    g("Netherlands", "NED", Uefa),
    g("New Caledonia", "NCL", Ofc),
    g("New Zealand", "NZL", Ofc),
    g("Nicaragua", "NCA", Concacaf),
    g("Niger", "NIG", Caf),
    g("Nigeria", "NGA", Caf),
    g("North Macedonia", "MKD", Uefa),
    g("Northern Ireland", "NIR", Uefa),
    g("Norway", "NOR", Uefa),
    g("Oman", "OMA", Afc),
    g("Pakistan", "PAK", Afc),
    g("Palestine", "PLE", Afc),
    g("Panama", "PAN", Concacaf),
    g("Papua New Guinea", "PNG", Ofc),
    g("Paraguay", "PAR", Conmebol),
    g("Peru", "PER", Conmebol),
    g("Philippines", "PHI", Afc),
    g("Poland", "POL", Uefa),
    g("Portugal", "POR", Uefa),
    g("Puerto Rico", "PUR", Concacaf),
    g("Qatar", "QAT", Afc),
    g("Reunion", "REU", Caf),
    g("Romania", "ROU", Uefa),
    g("Russia", "RUS", Uefa),
    g("Rwanda", "RWA", Caf),
    g("Saint-Martin", "MAF", Concacaf),
    g("Samoa", "SAM", Ofc),
    g("American Samoa", "ASA", Ofc),
    g("San Marino", "SMR", Uefa),
    g("Sao Tome and Principe", "STP", Caf),
    g("Saudi Arabia", "KSA", Afc),
    g("Scotland", "SCO", Uefa),
    g("Senegal", "SEN", Caf),
    g("Serbia", "SRB", Uefa),
    g("Seychelles", "SEY", Caf),
    g("Sierra Leone", "SLE", Caf),
    g("Singapore", "SIN", Afc),
    g("Sint Maarten", "SXM", Concacaf),
    g("Slovakia", "SVK", Uefa),
    g("Slovenia", "SVN", Uefa),
    g("Somalia", "SOM", Caf),
    g("South Africa", "RSA", Caf),
    g("South Sudan", "SSD", Caf),
    g("Spain", "ESP", Uefa),
    g("Sri Lanka", "SRI", Afc),
    g("St. Kitts & Nevis", "SKN", Concacaf),
    g("St. Lucia", "LCA", Concacaf),
    g("St. Vincent & Grenadinen", "VIN", Concacaf),
    g("Sudan", "SDN", Caf),
    g("Suriname", "SUR", Concacaf),
    g("Sweden", "SWE", Uefa),
    g("Switzerland", "SUI", Uefa),
    g("Syria", "SYR", Afc),
    g("Tahiti", "TAH", Ofc),
    g("Tajikistan", "TJK", Afc),
    g("Tanzania", "TAN", Caf),
    g("Thailand", "THA", Afc),
    g("The Gambia", "GAM", Caf),
    g("Timor-Leste", "TLS", Afc),
    g("Togo", "TOG", Caf),
    g("Tonga", "TGA", Ofc),
    g("Trinidad and Tobago", "TRI", Concacaf),
    g("Tunisia", "TUN", Caf),
    g("Turkey", "TUR", Uefa),
    g("Turkmenistan", "TKM", Afc),
    g("Tuvalu", "TUV", Ofc),
    g("Uganda", "UGA", Caf),
    g("Ukraine", "UKR", Uefa),
    g("United Arab Emirates", "UAE", Afc),
    g("United States", "USA", Concacaf),
    g("Uruguay", "URU", Conmebol),
    g("Uzbekistan", "UZB", Afc),
    g("Vanuatu", "VAN", Ofc),
    g("Venezuela", "VEN", Conmebol),
    g("Vietnam", "VIE", Afc),
    g("Wales", "WAL", Uefa),
    g("Yemen", "YEM", Afc),
    g("Zambia", "ZAM", Caf),
    g("Zimbabwe", "ZIM", Caf),
];

/// Other spellings of the same country, all folded. Two spellings in one source must never make two nations.
const ALIASES: &[(&str, &str)] = &[
    ("turkiye", "turkey"),
    ("neukaledonien", "new caledonia"),
    ("hong kong", "hongkong"),
    ("czechia", "czech republic"),
    ("cabo verde", "cape verde"),
    ("ivory coast", "cote d'ivoire"),
    ("bosnia and herzegovina", "bosnia-herzegovina"),
    ("brunei", "brunei darussalam"),
    ("gambia", "the gambia"),
    ("republic of ireland", "ireland"),
    ("south korea", "korea, south"),
    ("north korea", "korea, north"),
    ("southern sudan", "south sudan"),
    ("macau", "macao"),
    ("timor leste", "timor-leste"),
    ("congo dr", "dr congo"),
    ("usa", "united states"),
    ("st. vincent and the grenadines", "st. vincent & grenadinen"),
    ("saint kitts and nevis", "st. kitts & nevis"),
    ("saint lucia", "st. lucia"),
    ("sao tome e principe", "sao tome and principe"),
];

/// Lower case with accents removed, so `Réunion`, `Reunion` and `REUNION` are one name.
pub fn fold(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ø' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// The country a name refers to, through aliases and accent folding.
pub fn lookup(name: &str) -> Option<&'static Geo> {
    let f = fold(name);
    let f = ALIASES.iter().find(|(a, _)| *a == f).map_or(f, |(_, c)| (*c).to_string());
    COUNTRIES.iter().find(|c| fold(c.name) == f)
}

pub fn by_code(code: &str) -> Option<&'static Geo> {
    COUNTRIES.iter().find(|c| c.code.eq_ignore_ascii_case(code))
}

/// TM confederation words.
pub fn confed_from_tm(s: &str) -> Option<Confed> {
    match s.trim().to_ascii_lowercase().as_str() {
        "europa" | "uefa" => Some(Uefa),
        "afrika" | "caf" => Some(Caf),
        "asien" | "afc" => Some(Afc),
        "ozeanien" | "ofc" => Some(Ofc),
        _ => None,
    }
}

/// The staff list gives nations in French. Names spelled the same in English are not listed.
const FRENCH: &[(&str, &str)] = &[
    ("Italie", "Italy"),
    ("Espagne", "Spain"),
    ("Angleterre", "England"),
    ("Argentine", "Argentina"),
    ("Brésil", "Brazil"),
    ("Pays de Galles", "Wales"),
    ("Allemagne", "Germany"),
    ("Pays-Bas", "Netherlands"),
    ("Algérie", "Algeria"),
    ("Écosse", "Scotland"),
    ("États-Unis", "United States"),
    ("Irlande", "Ireland"),
    ("Croatie", "Croatia"),
    ("Maroc", "Morocco"),
    ("Belgique", "Belgium"),
    ("Serbie", "Serbia"),
    ("Cameroun", "Cameroon"),
    ("Irlande du Nord", "Northern Ireland"),
    ("Autriche", "Austria"),
    ("Côte d'Ivoire", "Cote d'Ivoire"),
    ("Sénégal", "Senegal"),
    ("Russie", "Russia"),
    ("Mexique", "Mexico"),
    ("Tunisie", "Tunisia"),
    ("Jamaïque", "Jamaica"),
    ("Pologne", "Poland"),
    ("Turquie", "Turkey"),
    ("Roumanie", "Romania"),
    ("Bosnie", "Bosnia-Herzegovina"),
    ("Colombie", "Colombia"),
    ("Chine", "China"),
    ("Danemark", "Denmark"),
    ("Suisse", "Switzerland"),
    ("République tchèque", "Czech Republic"),
    ("Thaïlande", "Thailand"),
    ("Suède", "Sweden"),
    ("Australie", "Australia"),
    ("La Réunion", "Reunion"),
    ("Albanie", "Albania"),
    ("R.D. Congo", "DR Congo"),
    ("Slovénie", "Slovenia"),
    ("Chili", "Chile"),
    ("Grèce", "Greece"),
    ("Bulgarie", "Bulgaria"),
    ("Monténégro", "Montenegro"),
    ("Finlande", "Finland"),
    ("Arménie", "Armenia"),
    ("Malaisie", "Malaysia"),
    ("Équateur", "Ecuador"),
    ("Arabie saoudite", "Saudi Arabia"),
    ("Cap Vert", "Cape Verde"),
    ("Mauritanie", "Mauritania"),
    ("Afrique du Sud", "South Africa"),
    ("Inde", "India"),
    ("Surinam", "Suriname"),
    ("Japon", "Japan"),
    ("Singapour", "Singapore"),
    ("Guinée Éq.", "Equatorial Guinea"),
    ("Hongrie", "Hungary"),
    ("Chypre", "Cyprus"),
    ("Indonésie", "Indonesia"),
    ("Guyane", "French Guiana"),
    ("Guinée", "Guinea"),
    ("Vénézuela", "Venezuela"),
    ("Islande", "Iceland"),
    ("Corée du Sud", "Korea, South"),
    ("Trinité & Tobago", "Trinidad and Tobago"),
    ("Pérou", "Peru"),
    ("Biélorussie", "Belarus"),
    ("Norvège", "Norway"),
    ("E.A.U.", "United Arab Emirates"),
    ("Andorre", "Andorra"),
    ("Angola", "Angola"),
    ("Israël", "Israel"),
    ("Comores", "Comoros"),
    ("Saint-Marin", "San Marino"),
    ("Haïti", "Haiti"),
    ("Nouvelle-Zélande", "New Zealand"),
    ("Malte", "Malta"),
    ("Lituanie", "Lithuania"),
    ("Nlle-Calédonie", "New Caledonia"),
    ("Centrafrique", "Central African Republic"),
    ("Somalie", "Somalia"),
    ("Slovaquie", "Slovakia"),
    ("Bénin", "Benin"),
    ("Antigua", "Antigua and Barbuda"),
    ("São Tomé & Principe", "Sao Tome and Principe"),
    ("Sainte-Lucie", "St. Lucia"),
    ("Barbades", "Barbados"),
    ("Tchad", "Chad"),
    ("Grenade", "Grenada"),
    ("Irak", "Iraq"),
    ("Macédoine du N.", "North Macedonia"),
    ("Bermudes", "Bermuda"),
    ("Curaçao", "Curacao"),
    ("Guinée-Bissau", "Guinea-Bissau"),
    ("Érythrée", "Eritrea"),
    ("Libye", "Libya"),
    ("Tadjikistan", "Tajikistan"),
    ("Turkménistan", "Turkmenistan"),
    ("Saint-Vincent", "St. Vincent & Grenadinen"),
    ("Égypte", "Egypt"),
    ("Saint-Kitts", "St. Kitts & Nevis"),
    ("Corée du Nord", "Korea, North"),
    ("Namibie", "Namibia"),
    ("Ouganda", "Uganda"),
    ("Éthiopie", "Ethiopia"),
    ("Azerbaïdjan", "Azerbaijan"),
    ("Estonie", "Estonia"),
    ("Moldavie", "Moldova"),
    ("Bolivie", "Bolivia"),
    ("Samoa Am.", "American Samoa"),
    ("Îles Vierges", ""),
    ("Îles Vierges US", ""),
    ("Mayotte", ""),
    ("Vietnam du Sud", ""),
    ("Pays Basque", ""),
];

/// A French nation token from the staff list: `Some(country)`, or `None` for regions, historical states and
/// territories with no country in the table (those stay UNKNOWN rather than being guessed).
pub fn french_country(token: &str) -> Option<&'static Geo> {
    let f = fold(token);
    match FRENCH.iter().find(|(fr, _)| fold(fr) == f) {
        Some((_, "")) => None,
        Some((_, en)) => lookup(en),
        None => lookup(token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_names_fold_uniquely() {
        let mut codes: Vec<_> = COUNTRIES.iter().map(|c| c.code).collect();
        codes.sort_unstable();
        let n = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), n, "duplicate country code");
        let mut names: Vec<_> = COUNTRIES.iter().map(|c| fold(c.name)).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "duplicate country name");
    }

    #[test]
    fn spellings_of_one_country_are_one_nation() {
        assert_eq!(lookup("Türkiye").unwrap().code, lookup("Turkey").unwrap().code);
        assert_eq!(lookup("Neukaledonien").unwrap().code, lookup("New Caledonia").unwrap().code);
        assert_eq!(lookup("Réunion").unwrap().code, "REU");
        assert_eq!(lookup("hongkong").unwrap().code, "HKG");
        assert!(lookup("Atlantis").is_none());
    }

    #[test]
    fn french_names_resolve_and_regions_stay_unknown() {
        assert_eq!(french_country("Espagne").unwrap().code, "ESP");
        assert_eq!(french_country("Écosse").unwrap().code, "SCO");
        assert_eq!(french_country("Ecosse").unwrap().code, "SCO");
        assert_eq!(french_country("Portugal").unwrap().code, "POR");
        assert!(french_country("Pays Basque").is_none());
        assert!(french_country("Vietnam du Sud").is_none());
        assert!(french_country("Nulle Part").is_none());
    }

    #[test]
    fn every_french_target_exists() {
        for (fr, en) in FRENCH {
            assert!(en.is_empty() || lookup(en).is_some(), "{fr} -> {en} is not in the country table");
        }
    }
}
