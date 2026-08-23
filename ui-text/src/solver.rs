//! The words the application speaks about a solve's configuration.
//!
//! One French sentence per [`collomatique_strategies::ConductorWarning`]
//! variant, phrased the way the application's own solve dialog shows it
//! (`gtk4/src/editor/run_solver/conductor_config.rs`), kept here so the python
//! module's warnings say what the dialog says. The match is exhaustive with no
//! wildcard arm, so a new warning over there is a compile error here — the
//! [`crate::caveats::caveat_text`] shape.

use collomatique_strategies::ConductorWarning;

/// The sentence a conductor warning is shown as
///
/// Flat, one line, no bullet: the dialog's list layout is its own, so a script
/// printing a warning gets a single sentence.
pub fn conductor_warning_text(warning: ConductorWarning) -> &'static str {
    match warning {
        ConductorWarning::NoStrategyEnabled => {
            "Aucune stratégie n'est activée : rien ne sera exécuté."
        }
        ConductorWarning::NoOptimizing => {
            "Aucune stratégie d'optimisation n'est activée : le solveur cherchera une solution \
             réalisable sans tenter de l'améliorer."
        }
        ConductorWarning::NoSeed => {
            "L'exploration aléatoire est activée mais aucune stratégie ne produit de solution \
             initiale (démarrage à chaud ou résolution incrémentale) : elle ne démarrera jamais et \
             le solveur s'arrêtera immédiatement."
        }
        ConductorWarning::StarvedFuzzy => {
            "L'exploration aléatoire est activée mais l'unique tâche est occupée par la stratégie \
             par défaut : elle n'aura jamais de créneau libre. Augmentez le nombre de tâches en \
             parallèle."
        }
        ConductorWarning::WontFinish => {
            "La stratégie par défaut est désactivée : sans elle, aucune borne ne prouve \
             l'optimalité et le solveur tournera indéfiniment."
        }
        ConductorWarning::ColdFuzzy => {
            "L'exploration aléatoire est activée sans solution initiale (démarrage à chaud ou \
             résolution incrémentale) : elle ne se déclenchera qu'une fois la stratégie par défaut \
             bien avancée et sera donc souvent inutile."
        }
        ConductorWarning::RedundantWarmStart => {
            "Le démarrage à chaud et la résolution incrémentale sont tous deux activés : la \
             résolution incrémentale fournit généralement un meilleur point de départ ; le \
             démarrage à chaud n'est utile que pour obtenir rapidement une solution."
        }
        ConductorWarning::OverwhelmedCpu => {
            "Le nombre de tâches en parallèle dépasse le nombre de cœurs du processeur."
        }
    }
}
