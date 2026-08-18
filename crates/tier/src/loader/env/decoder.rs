use std::collections::BTreeMap;

use crate::EnvDecoder;
use crate::path::path_matches_pattern;

use super::super::CustomEnvDecoder;

pub(super) fn custom_env_decoder_for_path<'a>(
    path: &str,
    custom_env_decoders: &'a BTreeMap<String, CustomEnvDecoder>,
) -> Option<&'a CustomEnvDecoder> {
    let mut best = None::<((usize, usize, Vec<bool>), &CustomEnvDecoder)>;

    for (pattern, decoder) in custom_env_decoders {
        if !path_matches_pattern(path, pattern) {
            continue;
        }

        let score = decoder_match_score(pattern);
        match &mut best {
            Some((best_score, best_decoder)) if score > *best_score => {
                *best_score = score;
                *best_decoder = decoder;
            }
            None => best = Some((score, decoder)),
            _ => {}
        }
    }

    best.map(|(_, decoder)| decoder)
}

pub(super) fn env_decoder_for_path(
    path: &str,
    env_decoders: &BTreeMap<String, EnvDecoder>,
) -> Option<EnvDecoder> {
    let mut best = None::<((usize, usize, Vec<bool>), EnvDecoder)>;

    for (pattern, decoder) in env_decoders {
        if !path_matches_pattern(path, pattern) {
            continue;
        }

        let score = decoder_match_score(pattern);
        match &mut best {
            Some((best_score, best_decoder)) if score > *best_score => {
                *best_score = score;
                *best_decoder = *decoder;
            }
            None => best = Some((score, *decoder)),
            _ => {}
        }
    }

    best.map(|(_, decoder)| decoder)
}

fn decoder_match_score(pattern: &str) -> (usize, usize, Vec<bool>) {
    let segments = pattern.split('.').collect::<Vec<_>>();
    (
        segments.len(),
        segments.iter().filter(|segment| **segment != "*").count(),
        segments.iter().map(|segment| *segment != "*").collect(),
    )
}
