//! Langues de la traduction à la volée.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslationLanguage {
    #[default]
    En,
    Es,
    De,
    It,
    Pt,
}

impl TranslationLanguage {
    pub const ALL: [TranslationLanguage; 5] = [Self::En, Self::Es, Self::De, Self::It, Self::Pt];

    pub fn label(self) -> &'static str {
        match self {
            Self::En => "Anglais",
            Self::Es => "Espagnol",
            Self::De => "Allemand",
            Self::It => "Italien",
            Self::Pt => "Portugais (Brésil)",
        }
    }

    /// Le nom de la langue en anglais (« English »), pour l'interface en anglais.
    pub fn name_en(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Spanish",
            Self::De => "German",
            Self::It => "Italian",
            Self::Pt => "Portuguese",
        }
    }

    /// Nom court (« Collé en anglais »).
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "anglais",
            Self::Es => "espagnol",
            Self::De => "allemand",
            Self::It => "italien",
            Self::Pt => "portugais",
        }
    }

    /// Nom dans la consigne du modèle (« traduis en portugais du Brésil »).
    pub fn prompt_name(self) -> &'static str {
        if self == Self::Pt {
            "portugais du Brésil"
        } else {
            self.name()
        }
    }

    /// Les exemples de [`crate::cleaner::TRANSLATION_EXAMPLES`], traduits dans cette langue (même ordre).
    pub fn examples(self) -> [&'static str; 7] {
        match self {
            Self::En => [
                "Can you summarize this document in three points?",
                "What's the difference between an API and an SDK?",
                "Write me an email telling Marc that the meeting is on Friday.",
                "I reread the contract yesterday and everything works for me, except article 3.",
                "The delivery (if all goes well) arrives on Monday.\nThanks again!!!",
                "The train leaves at 9 tomorrow morning.",
                "We'll meet on Tuesday in front of the town hall.",
            ],
            Self::Es => [
                "¿Puedes resumirme este documento en tres puntos?",
                "¿Cuál es la diferencia entre una API y un SDK?",
                "Escríbeme un correo para decirle a Marc que la cita es el viernes.",
                "Releí el contrato ayer y todo me parece bien, excepto el artículo 3.",
                "La entrega (si todo va bien) llega el lunes.\n¡¡¡Gracias de nuevo!!!",
                "El tren sale mañana por la mañana a las 9.",
                "Nos vemos el martes delante del ayuntamiento.",
            ],
            Self::De => [
                "Kannst du mir dieses Dokument in drei Punkten zusammenfassen?",
                "Was ist der Unterschied zwischen einer API und einem SDK?",
                "Schreib mir eine E-Mail, um Marc zu sagen, dass der Termin am Freitag ist.",
                "Ich habe den Vertrag gestern noch einmal gelesen, und alles passt mir, außer Artikel 3.",
                "Die Lieferung (wenn alles gut geht) kommt am Montag.\nNochmals danke!!!",
                "Der Zug fährt morgen früh um 9 Uhr ab.",
                "Wir treffen uns am Dienstag vor dem Rathaus.",
            ],
            Self::It => [
                "Puoi riassumermi questo documento in tre punti?",
                "Qual è la differenza tra un'API e un SDK?",
                "Scrivimi una mail per dire a Marc che l'appuntamento è venerdì.",
                "Ho riletto il contratto ieri e mi va tutto bene, tranne l'articolo 3.",
                "La consegna (se tutto va bene) arriva lunedì.\nGrazie ancora!!!",
                "Il treno parte domani mattina alle 9.",
                "Ci vediamo martedì davanti al municipio.",
            ],
            Self::Pt => [
                "Você pode resumir este documento em três pontos?",
                "Qual é a diferença entre uma API e um SDK?",
                "Me escreva um e-mail dizendo ao Marc que a reunião é na sexta-feira.",
                "Reli o contrato ontem e está tudo certo para mim, exceto o artigo 3.",
                "A entrega (se tudo der certo) chega na segunda-feira.\nObrigado de novo!!!",
                "O trem sai amanhã de manhã às 9h.",
                "A gente se encontra na terça-feira em frente à prefeitura.",
            ],
        }
    }
}
