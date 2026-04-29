// src-tauri/src/goodies.rs
//
// Tageszitate / paedagogische Aphorismen, die beim Login oder im Dashboard
// als kleines Banner erscheinen koennen. Steuerbar ueber config.toml
// (`[goodies] enabled = true|false`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Zitat {
    pub text: String,
    pub autor: String,
}

const ZITATE: &[(&str, &str)] = &[
    ("Was du mir sagst, das vergesse ich. Was du mir zeigst, daran erinnere ich mich. Was du mich tun lässt, das verstehe ich.", "Konfuzius"),
    ("Lernen ist wie Rudern gegen den Strom. Hört man damit auf, treibt man zurück.", "Laotse"),
    ("Ein Kind ist kein Gefäß, das gefüllt, sondern ein Feuer, das entzündet werden will.", "Plutarch"),
    ("Du kannst einem Menschen nichts lehren, du kannst ihm nur helfen, es in sich selbst zu entdecken.", "Galileo Galilei"),
    ("Erziehung ist Beispiel und Liebe, sonst nichts.", "Friedrich Fröbel"),
    ("Ein Kind wächst in dem Maße, in dem ihm vertraut wird.", "Janusz Korczak"),
    ("Pädagogik ist nichts ohne Mut.", "Janusz Korczak"),
    ("Erziehung soll dem Menschen Flügel geben, ohne ihn vom Boden zu lösen.", "Maria Montessori"),
    ("Hilf mir, es selbst zu tun.", "Maria Montessori"),
    ("Es gibt nichts Gutes, außer man tut es.", "Erich Kästner"),
    ("Ein guter Lehrer hat nichts dagegen, vom Schüler übertroffen zu werden.", "Friedrich Nietzsche"),
    ("Schule ist nicht Vorbereitung auf das Leben — Schule ist das Leben.", "John Dewey"),
    ("Drei Dinge sind uns aus dem Paradies geblieben: die Sterne der Nacht, die Blumen des Tages und die Augen der Kinder.", "Dante Alighieri"),
    ("Der Lehrer kann nur die Tür öffnen — eintreten musst du selbst.", "Chinesisches Sprichwort"),
    ("Geduld ist ein Baum mit bitteren Wurzeln, aber süßen Früchten.", "Persisches Sprichwort"),
    ("Es gibt keinen Weg zum Glück. Glücklichsein ist der Weg.", "Buddha"),
    ("Mache aus jedem Tag dein Meisterwerk.", "John Wooden"),
    ("Wer aufhört, besser zu werden, hat aufgehört, gut zu sein.", "Marie von Ebner-Eschenbach"),
    ("Die größte Entdeckung meiner Generation ist, dass Menschen ihr Leben ändern können, indem sie ihre Einstellung ändern.", "William James"),
    ("Wer ein Warum zum Leben hat, erträgt fast jedes Wie.", "Friedrich Nietzsche"),
    ("Ein Lehrer, der vorgibt zu wissen, was er nicht weiß, hat aufgehört, ein Lehrer zu sein.", "Sokrates"),
    ("Niemand wird als Lehrer geboren, aber jeder Lehrer war einmal Schüler.", "Pädagogische Volksweisheit"),
    ("Bildung ist das, was übrig bleibt, wenn man alles vergessen hat, was man in der Schule gelernt hat.", "Albert Einstein"),
    ("Phantasie ist wichtiger als Wissen, denn Wissen ist begrenzt.", "Albert Einstein"),
    ("Worte können Mut machen oder Mut nehmen — wähle sorgfältig.", "Anonym"),
    ("Wer Kinder begleitet, lernt zweimal.", "Pädagogische Volksweisheit"),
    ("Eine Note misst, was leicht messbar ist — nicht, was zählt.", "Anonym"),
    ("Lerne von gestern, lebe für heute, hoffe für morgen.", "Albert Einstein"),
    ("Es ist nicht genug zu wissen — man muss auch anwenden. Es ist nicht genug zu wollen — man muss auch tun.", "Johann Wolfgang von Goethe"),
    ("Behandle die Menschen so, als wären sie, was sie sein sollten, und du hilfst ihnen zu werden, was sie sein können.", "Johann Wolfgang von Goethe"),
];

/// Pseudo-zufaellige Auswahl auf Basis der aktuellen Tageszeit (UNIX-Sekunden).
/// Gibt bei wiederholtem Aufruf innerhalb derselben Sekunde dasselbe Zitat —
/// das ist gewuenscht, damit Refresh nicht zum Wuerfel-Spiel wird.
pub fn random_zitat(now_secs: u64) -> Zitat {
    let idx = (now_secs as usize) % ZITATE.len();
    let (text, autor) = ZITATE[idx];
    Zitat { text: text.to_string(), autor: autor.to_string() }
}

/// Wrapper, der die System-Uhr abfragt. In Tests nutze die seed-Funktion oben.
pub fn aktuelles_zitat() -> Zitat {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Tagesgranularitaet: 86400s pro Tag, damit ein Zitat den ganzen Tag ueberlebt.
    random_zitat(secs / 86400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_zitat_ist_deterministisch_pro_seed() {
        let a = random_zitat(42);
        let b = random_zitat(42);
        assert_eq!(a, b);
    }

    #[test]
    fn random_zitat_variiert_mit_seed() {
        let mut gesehen = std::collections::HashSet::new();
        for s in 0..ZITATE.len() as u64 {
            gesehen.insert(random_zitat(s).text);
        }
        assert_eq!(gesehen.len(), ZITATE.len(), "jeder Seed sollte ein anderes Zitat liefern");
    }

    #[test]
    fn alle_zitate_haben_text_und_autor() {
        for (text, autor) in ZITATE {
            assert!(!text.is_empty(), "leerer Text: {autor}");
            assert!(!autor.is_empty(), "leerer Autor: {text}");
        }
    }

    #[test]
    fn aktuelles_zitat_lieferbar() {
        let z = aktuelles_zitat();
        assert!(!z.text.is_empty());
        assert!(!z.autor.is_empty());
    }
}
