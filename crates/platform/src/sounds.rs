//! Sons de début et de fin de dictée.

pub trait Sounds: Send + Sync {
    /// Joue le son `name` (celui des réglages) à `volume` (0 à 1) sans attendre sa fin. « Aucun son » ne joue rien.
    fn play(&self, name: &str, volume: f64);
}

pub struct Silent;

impl Sounds for Silent {
    fn play(&self, _: &str, _: f64) {}
}
