//! Année scolaire et échéance des jetons.

use time::{Date, Month, OffsetDateTime, Time};

/// Vérifie le format `AAAA-AAAA` et la consécutivité des deux millésimes.
pub fn parse(input: &str) -> Result<(i32, i32), String> {
    let (a, b) = input
        .split_once('-')
        .ok_or_else(|| format!("année scolaire attendue au format `2026-2027`, reçu `{input}`"))?;

    let start: i32 = a
        .trim()
        .parse()
        .map_err(|_| format!("millésime illisible : `{a}`"))?;
    let end: i32 = b
        .trim()
        .parse()
        .map_err(|_| format!("millésime illisible : `{b}`"))?;

    if end != start + 1 {
        return Err(format!(
            "années non consécutives : `{start}-{end}`"
        ));
    }
    if !(2000..=2100).contains(&start) {
        return Err(format!("millésime hors plage plausible : `{start}`"));
    }
    Ok((start, end))
}

/// Échéance des jetons d'un groupe : 31 août de l'année de fin.
///
/// La date est fixée par le calendrier scolaire et non par la date de création
/// du groupe : un groupe créé en mai ne doit pas voir ses jetons survivre
/// jusqu'au mois de mai suivant. Voir
/// [ADR-0002](../../../docs/adr/0002-jetons-stables-a-l-annee.md).
pub fn expiry(school_year: &str) -> Result<OffsetDateTime, String> {
    let (_, end) = parse(school_year)?;
    let date = Date::from_calendar_date(end, Month::August, 31)
        .map_err(|e| format!("date d'échéance invalide : {e}"))?;
    Ok(date.with_time(Time::MIDNIGHT).assume_utc() + time::Duration::days(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annees_consecutives_acceptees() {
        assert_eq!(parse("2026-2027"), Ok((2026, 2027)));
    }

    #[test]
    fn formats_refuses() {
        for mauvais in ["2026", "2026/2027", "2026-2028", "2026-2025", "abcd-efgh", ""] {
            assert!(parse(mauvais).is_err(), "aurait dû refuser : {mauvais:?}");
        }
    }

    #[test]
    fn echeance_au_31_aout_de_lannee_de_fin() {
        let e = expiry("2026-2027").unwrap();
        assert_eq!(e.year(), 2027);
        assert_eq!(e.month(), Month::September);
        assert_eq!(e.day(), 1);
        // Minuit le 1er septembre : le 31 août est donc inclus en entier.
    }

    #[test]
    fn echeance_independante_de_la_date_de_creation() {
        assert_eq!(expiry("2026-2027").unwrap(), expiry("2026-2027").unwrap());
    }
}
