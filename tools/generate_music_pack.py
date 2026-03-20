from __future__ import annotations

import json
import math
import os
import random
import shutil
import struct
import subprocess
import wave
from array import array
from dataclasses import dataclass
from pathlib import Path

PPQ = 480
BAR_TICKS = PPQ * 4
SAMPLE_RATE = 16_000
TABLE_SIZE = 2048
TABLE_MASK = TABLE_SIZE - 1


def _make_noise_table() -> list[float]:
    rng = random.Random(2000)
    return [rng.uniform(-1.0, 1.0) for _ in range(TABLE_SIZE)]


SINE_TABLE = [math.sin(2.0 * math.pi * i / TABLE_SIZE) for i in range(TABLE_SIZE)]
TRI_TABLE = [2.0 * abs(2.0 * (i / TABLE_SIZE) - 1.0) - 1.0 for i in range(TABLE_SIZE)]
SAW_TABLE = [2.0 * (i / TABLE_SIZE) - 1.0 for i in range(TABLE_SIZE)]
NOISE_TABLE = _make_noise_table()

SCALES = {
    "major": [0, 2, 4, 5, 7, 9, 11],
    "lydian": [0, 2, 4, 6, 7, 9, 11],
    "mixolydian": [0, 2, 4, 5, 7, 9, 10],
    "dorian": [0, 2, 3, 5, 7, 9, 10],
    "aeolian": [0, 2, 3, 5, 7, 8, 10],
}

CHORDS = {
    "maj7": [0, 4, 7, 11],
    "maj9": [0, 4, 7, 11, 14],
    "add9": [0, 4, 7, 14],
    "six_nine": [0, 4, 7, 9, 14],
    "m7": [0, 3, 7, 10],
    "m9": [0, 3, 7, 10, 14],
    "sus2": [0, 2, 7, 14],
    "sus4": [0, 5, 7, 10],
    "dom9": [0, 4, 7, 10, 14],
}


def vlq(value: int) -> bytes:
    buffer = [value & 0x7F]
    value >>= 7
    while value:
        buffer.append((value & 0x7F) | 0x80)
        value >>= 7
    buffer.reverse()
    return bytes(buffer)


class Track:
    def __init__(self, name: str = "") -> None:
        self.name = name
        self.events: list[tuple[int, int, bytes]] = []
        self._order = 0

    def add(self, tick: int, data: bytes, priority: int = 1) -> None:
        self.events.append((tick, priority * 100000 + self._order, data))
        self._order += 1

    def meta(self, tick: int, meta_type: int, payload: bytes) -> None:
        self.add(tick, bytes([0xFF, meta_type]) + vlq(len(payload)) + payload)

    def text(self, tick: int, text: str) -> None:
        self.meta(tick, 0x03, text.encode("ascii"))

    def tempo(self, tick: int, bpm: int) -> None:
        mpqn = round(60_000_000 / bpm)
        self.meta(tick, 0x51, mpqn.to_bytes(3, "big"))

    def time_signature(self, tick: int, numerator: int = 4, denominator: int = 4) -> None:
        dd = {1: 0, 2: 1, 4: 2, 8: 3}[denominator]
        self.meta(tick, 0x58, bytes([numerator, dd, 24, 8]))

    def program(self, tick: int, channel: int, program: int) -> None:
        self.add(tick, bytes([0xC0 | channel, program & 0x7F]))

    def control(self, tick: int, channel: int, control: int, value: int) -> None:
        self.add(tick, bytes([0xB0 | channel, control & 0x7F, value & 0x7F]))

    def note(self, start: int, duration: int, channel: int, note: int, velocity: int) -> None:
        self.add(start, bytes([0x90 | channel, note & 0x7F, velocity & 0x7F]), priority=2)
        self.add(
            start + max(1, duration),
            bytes([0x80 | channel, note & 0x7F, 0]),
            priority=0,
        )

    def render(self, end_tick: int) -> bytes:
        self.meta(end_tick, 0x2F, b"")
        if self.name:
            self.text(0, self.name)
        body = bytearray()
        last_tick = 0
        for tick, _, data in sorted(self.events, key=lambda item: (item[0], item[1])):
            delta = tick - last_tick
            body.extend(vlq(delta))
            body.extend(data)
            last_tick = tick
        return b"MTrk" + struct.pack(">I", len(body)) + bytes(body)


def scale_note(key_root: int, scale_name: str, degree: int, octave_shift: int = 0) -> int:
    scale = SCALES[scale_name]
    idx = (degree - 1) % len(scale)
    octave = (degree - 1) // len(scale)
    return key_root + scale[idx] + 12 * (octave + octave_shift)


def chord_tones(key_root: int, root_offset: int, quality: str, low_octave: int = 0) -> list[int]:
    root = key_root + root_offset + 12 * low_octave
    return [root + interval for interval in CHORDS[quality]]


def beat(bar: int, beat_offset: float) -> int:
    return int(bar * BAR_TICKS + beat_offset * PPQ)


@dataclass(frozen=True)
class SongSpec:
    slug: str
    title: str
    role: str
    bpm: int
    key_root: int
    scale: str
    bars: int
    lead_program: int
    comp_program: int
    pad_program: int
    bass_program: int
    progression: tuple[tuple[int, str], ...]
    motif_a: tuple[tuple[float, float, int, int], ...]
    motif_b: tuple[tuple[float, float, int, int], ...]
    melody_stride: int = 2
    drums: bool = False
    description: str = ""


SONGS: tuple[SongSpec, ...] = (
    SongSpec(
        slug="01_civic_sunrise_theme",
        title="Civic Sunrise",
        role="start_theme",
        bpm=88,
        key_root=62,
        scale="lydian",
        bars=40,
        lead_program=10,
        comp_program=4,
        pad_program=89,
        bass_program=33,
        progression=((0, "maj9"), (7, "six_nine"), (9, "m9"), (4, "add9"), (2, "sus2"), (9, "m9"), (7, "maj7"), (4, "add9")),
        motif_a=((0.0, 1.0, 1, 1), (1.5, 0.5, 3, 1), (2.0, 1.0, 5, 1), (3.0, 0.75, 6, 1)),
        motif_b=((0.0, 0.75, 5, 1), (1.0, 0.5, 6, 1), (2.0, 1.0, 3, 2), (3.25, 0.5, 2, 2)),
        melody_stride=1,
        drums=True,
        description="Shorter, brighter title theme with a clearer hook and less ambient drift for the start screen.",
    ),
    SongSpec(
        slug="02_riverfront_reflections",
        title="Riverfront Reflections",
        role="gameplay",
        bpm=74,
        key_root=60,
        scale="major",
        bars=56,
        lead_program=71,
        comp_program=5,
        pad_program=88,
        bass_program=32,
        progression=((0, "maj9"), (5, "m9"), (9, "m7"), (7, "six_nine")),
        motif_a=((0.0, 1.5, 1, 1), (2.0, 0.75, 5, 1), (3.0, 0.75, 3, 1)),
        motif_b=((0.0, 1.0, 6, 1), (1.5, 0.5, 5, 1), (2.5, 1.0, 2, 1)),
        description="Gentle day-time city building cue with open piano voicings.",
    ),
    SongSpec(
        slug="03_tramline_twilight",
        title="Tramline Twilight",
        role="gameplay",
        bpm=82,
        key_root=65,
        scale="dorian",
        bars=56,
        lead_program=73,
        comp_program=11,
        pad_program=89,
        bass_program=35,
        progression=((0, "m9"), (5, "sus2"), (7, "maj7"), (3, "m7")),
        motif_a=((0.0, 0.5, 1, 1), (1.0, 0.5, 3, 1), (2.0, 1.0, 5, 1), (3.25, 0.5, 4, 1)),
        motif_b=((0.0, 0.75, 6, 1), (1.0, 0.75, 5, 1), (2.5, 0.5, 3, 1), (3.0, 0.5, 2, 1)),
        description="Soft dusk cue with a slightly urban, transit-adjacent motion.",
    ),
    SongSpec(
        slug="04_blueprint_breeze",
        title="Blueprint Breeze",
        role="gameplay",
        bpm=76,
        key_root=67,
        scale="mixolydian",
        bars=56,
        lead_program=74,
        comp_program=4,
        pad_program=90,
        bass_program=33,
        progression=((0, "add9"), (10, "sus2"), (5, "maj7"), (7, "dom9")),
        motif_a=((0.0, 1.0, 1, 1), (2.0, 0.5, 2, 2), (2.75, 0.75, 6, 1)),
        motif_b=((0.0, 1.25, 5, 1), (2.0, 0.5, 4, 1), (3.0, 0.75, 2, 1)),
        description="Airy planning music with light woodwind phrases.",
    ),
    SongSpec(
        slug="05_greenbelt_glow",
        title="Greenbelt Glow",
        role="gameplay",
        bpm=72,
        key_root=57,
        scale="major",
        bars=56,
        lead_program=68,
        comp_program=11,
        pad_program=88,
        bass_program=32,
        progression=((0, "six_nine"), (4, "add9"), (9, "m7"), (5, "maj9")),
        motif_a=((0.0, 1.5, 3, 1), (2.0, 0.75, 5, 1), (3.0, 0.5, 6, 1)),
        motif_b=((0.5, 0.75, 2, 1), (1.5, 0.75, 4, 1), (3.0, 0.75, 1, 2)),
        description="Pastoral and calm, suited to greener, low-density city moments.",
    ),
    SongSpec(
        slug="06_harbor_haze",
        title="Harbor Haze",
        role="gameplay",
        bpm=80,
        key_root=55,
        scale="dorian",
        bars=56,
        lead_program=65,
        comp_program=5,
        pad_program=90,
        bass_program=33,
        progression=((0, "m9"), (7, "maj7"), (10, "sus4"), (5, "m7")),
        motif_a=((0.0, 0.75, 1, 1), (1.0, 0.5, 5, 1), (2.0, 1.0, 6, 1), (3.25, 0.5, 3, 1)),
        motif_b=((0.0, 1.0, 4, 1), (1.5, 0.5, 2, 1), (2.5, 0.75, 1, 2)),
        description="A slightly moodier waterfront cue with a misty low-end.",
    ),
    SongSpec(
        slug="07_suburban_starlight",
        title="Suburban Starlight",
        role="gameplay",
        bpm=70,
        key_root=64,
        scale="lydian",
        bars=56,
        lead_program=14,
        comp_program=4,
        pad_program=89,
        bass_program=32,
        progression=((0, "maj9"), (2, "sus2"), (7, "six_nine"), (9, "m9")),
        motif_a=((0.0, 1.0, 5, 1), (1.5, 0.5, 6, 1), (2.0, 1.0, 3, 2)),
        motif_b=((0.0, 0.75, 2, 2), (1.0, 0.75, 1, 2), (2.5, 0.5, 6, 1)),
        description="Night-time residential ambience with soft bell colors.",
    ),
    SongSpec(
        slug="08_rain_on_concrete",
        title="Rain On Concrete",
        role="gameplay",
        bpm=68,
        key_root=61,
        scale="aeolian",
        bars=56,
        lead_program=70,
        comp_program=5,
        pad_program=91,
        bass_program=35,
        progression=((0, "m9"), (3, "m7"), (8, "maj7"), (10, "sus2")),
        motif_a=((0.0, 1.25, 1, 1), (2.0, 0.5, 3, 1), (3.0, 0.5, 5, 1)),
        motif_b=((0.5, 0.75, 6, 1), (1.5, 0.75, 4, 1), (3.0, 0.75, 2, 1)),
        description="A slower, overcast track with restrained melodic movement.",
    ),
    SongSpec(
        slug="09_market_square_nocturne",
        title="Market Square Nocturne",
        role="gameplay",
        bpm=84,
        key_root=59,
        scale="mixolydian",
        bars=56,
        lead_program=66,
        comp_program=11,
        pad_program=88,
        bass_program=33,
        progression=((0, "six_nine"), (5, "dom9"), (7, "maj7"), (10, "sus2")),
        motif_a=((0.0, 0.5, 1, 2), (1.0, 0.5, 7, 1), (2.0, 0.75, 5, 1), (3.0, 0.5, 4, 1)),
        motif_b=((0.0, 1.0, 3, 2), (1.5, 0.5, 2, 2), (2.5, 0.75, 1, 2)),
        description="A livelier but still mellow evening cue for busier downtown play.",
    ),
    SongSpec(
        slug="11_metropolitan_mists",
        title="Metropolitan Mists",
        role="gameplay",
        bpm=72,
        key_root=63,  # Eb
        scale="major",
        bars=48,
        lead_program=75,  # Pan Flute (breathy/relaxed)
        comp_program=5,   # Rhodes EP
        pad_program=90,   # Warm Pad
        bass_program=35,  # Fretless Bass
        progression=((0, "maj9"), (5, "dom9"), (8, "maj7"), (10, "sus4")),
        motif_a=(),
        motif_b=(),
        description="A hand-composed, non-algorithmic relaxing track with breathy woodwinds and jazzy electric piano.",
    ),
)

ACTIVE_SONG_SLUGS = (
    "01_civic_sunrise_theme",
    "11_metropolitan_mists",
)


def active_songs() -> tuple[SongSpec, ...]:
    return tuple(spec for spec in SONGS if spec.slug in ACTIVE_SONG_SLUGS)


@dataclass
class NoteEvent:
    start_tick: int
    duration_ticks: int
    note: int
    velocity: int


@dataclass
class AudioTrack:
    name: str
    kind: str
    channel: int
    program_number: int = 0
    has_program: bool = False
    volume: int = 100
    pan: int = 64
    reverb: int = 0
    notes: list[NoteEvent] | None = None

    def __post_init__(self) -> None:
        if self.notes is None:
            self.notes = []

    def program(self, _tick: int, _channel: int, program: int) -> None:
        self.program_number = program
        self.has_program = True

    def control(self, _tick: int, _channel: int, control: int, value: int) -> None:
        if control == 7:
            self.volume = value
        elif control == 10:
            self.pan = value
        elif control == 91:
            self.reverb = value

    def note(self, start: int, duration: int, _channel: int, note: int, velocity: int) -> None:
        self.notes.append(NoteEvent(start, duration, note, velocity))


@dataclass(frozen=True)
class ArrangementState:
    section: str
    intensity: float
    comp_pattern: int
    bass_pattern: int
    drum_pattern: int
    lead_active: bool
    pad_restrike: bool
    phrase_shift: int
    motif_variant: int


COMP_PATTERNS: tuple[tuple[tuple[float, float, int], ...], ...] = (
    (
        (0.0, 1.25, 0),
        (2.0, 0.75, 2),
        (3.0, 0.75, 1),
    ),
    (
        (0.0, 0.75, 0),
        (1.0, 0.5, 2),
        (1.5, 0.5, 1),
        (2.0, 0.75, 3),
        (3.0, 0.5, 1),
        (3.5, 0.35, 2),
    ),
    (
        (0.0, 0.5, 0),
        (0.75, 0.25, 1),
        (1.5, 0.5, 2),
        (2.0, 0.5, 3),
        (2.75, 0.25, 1),
        (3.0, 0.5, 2),
        (3.5, 0.35, 0),
    ),
)

BASS_PATTERNS: tuple[tuple[tuple[float, float, str], ...], ...] = (
    (
        (0.0, 1.5, "root"),
        (2.0, 1.0, "fifth"),
        (3.25, 0.5, "root"),
    ),
    (
        (0.0, 1.0, "root"),
        (1.5, 0.5, "fifth"),
        (2.0, 1.0, "octave"),
        (3.25, 0.5, "approach"),
    ),
    (
        (0.0, 0.75, "root"),
        (1.0, 0.5, "fifth"),
        (2.0, 0.75, "root"),
        (3.0, 0.5, "octave"),
        (3.5, 0.35, "approach"),
    ),
)


def stable_seed(text: str, salt: int = 0) -> int:
    value = 0x345678 + salt * 1009
    for idx, char in enumerate(text):
        value = (value * 131 + (idx + 1) * ord(char)) & 0xFFFFFFFF
    return value


def clamp_note(note: int) -> int:
    return max(28, min(108, note))


def humanize_offset(rng: random.Random, offset: float, amount: float = 0.035) -> float:
    shifted = offset + (rng.random() - 0.5) * amount
    return max(0.0, min(3.95, shifted))


def humanize_duration(rng: random.Random, duration: float, amount: float = 0.08) -> float:
    scaled = duration * (1.0 + (rng.random() - 0.5) * amount)
    return max(0.12, scaled)


def humanize_velocity(rng: random.Random, velocity: int, spread: int = 10) -> int:
    return max(18, min(110, velocity + rng.randint(-spread, spread)))


def add_humanized_note(
    track: Track | AudioTrack,
    bar: int,
    offset: float,
    duration: float,
    channel: int,
    note: int,
    velocity: int,
    rng: random.Random,
    timing: float = 0.035,
    velocity_spread: int = 8,
) -> None:
    start = beat(bar, humanize_offset(rng, offset, timing))
    duration_ticks = int(humanize_duration(rng, duration) * PPQ)
    track.note(start, max(1, duration_ticks), channel, clamp_note(note), humanize_velocity(rng, velocity, velocity_spread))


def mutate_motif(
    motif: tuple[tuple[float, float, int, int], ...],
    variant: int,
    rng: random.Random,
) -> list[tuple[float, float, int, int]]:
    events = [list(item) for item in motif]
    if variant == 1 and events:
        events[-1][2] += 1
        events[-1][1] *= 1.2
    elif variant == 2 and len(events) >= 2:
        events[1][0] = max(0.0, events[1][0] - 0.25)
        events[1][1] *= 0.75
        events[-1][2] -= 1
    elif variant == 3:
        for idx, event in enumerate(events):
            if idx % 2 == 1:
                event[2] += 1
            event[0] = max(0.0, min(3.5, event[0] + (rng.random() - 0.5) * 0.18))
    return [(float(offset), float(duration), int(degree), int(octave)) for offset, duration, degree, octave in events]


def voiced_chord(spec: SongSpec, chord: tuple[int, str], inversion: int, spread_up: bool) -> list[int]:
    root_offset, quality = chord
    notes = chord_tones(spec.key_root, root_offset, quality, low_octave=0)
    voicing = [notes[0] - 12, notes[1], notes[2], notes[3]]
    inversion = inversion % len(voicing)
    for idx in range(inversion):
        voicing[idx] += 12
    if spread_up and len(notes) > 4:
        voicing.append(notes[4] + 12)
    return [clamp_note(note) for note in sorted(voicing)]


def arrangement_state(spec: SongSpec, bar: int) -> ArrangementState:
    intro = 4
    bridge = 8
    outro = 4
    middle = max(8, spec.bars - intro - bridge - outro)
    verse_a = middle // 3
    verse_b = middle // 3
    finale = middle - verse_a - verse_b

    if bar < intro:
        return ArrangementState("intro", 0.28, 0, 0, 0, False, False, 0, 0)
    if bar < intro + verse_a:
        phrase = (bar - intro) // 4
        return ArrangementState("verse_a", 0.52, 1, 1, 0, bar % max(1, spec.melody_stride) == 0 and phrase % 2 == 1, phrase % 2 == 1, 0, phrase % 4)
    if bar < intro + verse_a + verse_b:
        phrase = (bar - intro - verse_a) // 4
        return ArrangementState("verse_b", 0.66, 1 + (phrase % 2), 1, 1 if spec.drums else 0, bar % max(1, spec.melody_stride) == 0, True, 1, (phrase + 1) % 4)
    if bar < intro + verse_a + verse_b + bridge:
        phrase = (bar - intro - verse_a - verse_b) // 2
        return ArrangementState("bridge", 0.42, 0, 0, 0, phrase % 2 == 1, False, 1, 2 + (phrase % 2))
    phrase = (bar - intro - verse_a - verse_b - bridge) // 4
    if bar >= spec.bars - outro:
        return ArrangementState("outro", 0.34, 0, 0, 0, False, False, 0, 0)
    return ArrangementState("finale", 0.82, 2, 2, 2 if spec.drums else 0, True, True, 1, (phrase + 2) % 4)


def resolve_bass_tone(current: tuple[int, str], following: tuple[int, str], role: str, spec: SongSpec) -> int:
    current_notes = chord_tones(spec.key_root, current[0], current[1], low_octave=-1)
    root = current_notes[0]
    fifth = current_notes[2]
    octave = root + 12
    next_root = chord_tones(spec.key_root, following[0], following[1], low_octave=-1)[0]
    if role == "root":
        return root
    if role == "fifth":
        return fifth
    if role == "octave":
        return octave
    step = -1 if next_root < root else 1
    return root + step


def add_pad(track: Track | AudioTrack, spec: SongSpec, bar: int, chord: tuple[int, str], state: ArrangementState, rng: random.Random) -> None:
    voicing = voiced_chord(spec, chord, inversion=(bar // 2) % 3, spread_up=state.intensity > 0.6)
    segments = [(0.0, 4.0)]
    if state.pad_restrike:
        segments = [(0.0, 2.0), (2.0, 2.0)]
    for seg_offset, seg_duration in segments:
        for idx, note in enumerate(voicing):
            add_humanized_note(
                track,
                bar,
                seg_offset + idx * 0.01,
                seg_duration - 0.05,
                2,
                note,
                int(42 + state.intensity * 18),
                rng,
                timing=0.02,
                velocity_spread=5,
            )


def add_comp(track: Track | AudioTrack, spec: SongSpec, bar: int, chord: tuple[int, str], state: ArrangementState, rng: random.Random) -> None:
    notes = chord_tones(spec.key_root, chord[0], chord[1], low_octave=1)
    pattern = COMP_PATTERNS[state.comp_pattern]
    for step_idx, (offset, duration, note_idx) in enumerate(pattern):
        note = notes[note_idx % len(notes)]
        if state.section == "bridge" and step_idx % 2 == 1:
            note += 12
        add_humanized_note(
            track,
            bar,
            offset,
            duration,
            1,
            note,
            int(48 + state.intensity * 24),
            rng,
        )


def add_bass(
    track: Track | AudioTrack,
    spec: SongSpec,
    progression: list[tuple[int, str]],
    bar: int,
    state: ArrangementState,
    rng: random.Random,
) -> None:
    current = progression[bar]
    following = progression[(bar + 1) % len(progression)]
    for offset, duration, role in BASS_PATTERNS[state.bass_pattern]:
        note = resolve_bass_tone(current, following, role, spec)
        add_humanized_note(
            track,
            bar,
            offset,
            duration,
            3,
            note,
            int(54 + state.intensity * 20),
            rng,
            timing=0.025,
            velocity_spread=6,
        )


def add_lead(
    track: Track | AudioTrack,
    spec: SongSpec,
    bar: int,
    state: ArrangementState,
    rng: random.Random,
) -> None:
    base = spec.motif_a if (bar // max(1, spec.melody_stride)) % 4 in (0, 1) else spec.motif_b
    motif = mutate_motif(base, state.motif_variant, rng)
    for idx, (offset, duration, degree, octave_shift) in enumerate(motif):
        if state.section == "bridge" and idx == 0:
            continue
        if state.section == "verse_a" and idx == len(motif) - 1 and rng.random() < 0.35:
            continue
        note = scale_note(spec.key_root, spec.scale, degree + state.phrase_shift, octave_shift)
        add_humanized_note(
            track,
            bar,
            offset,
            duration,
            0,
            note,
            int(58 + state.intensity * 22 + idx * 2),
            rng,
            timing=0.05,
            velocity_spread=10,
        )
    if state.section in {"verse_b", "finale"} and rng.random() < 0.5:
        pickup_degree = motif[-1][2] + state.phrase_shift + 1
        pickup_note = scale_note(spec.key_root, spec.scale, pickup_degree, motif[-1][3])
        add_humanized_note(track, bar, 3.5, 0.3, 0, pickup_note, int(52 + state.intensity * 20), rng, timing=0.03)


def add_drums(track: Track | AudioTrack, bar: int, state: ArrangementState, rng: random.Random) -> None:
    if state.drum_pattern == 0:
        if state.section == "intro" and bar % 2 == 0:
            add_humanized_note(track, bar, 0.0, 0.14, 9, 42, 24, rng, timing=0.015, velocity_spread=3)
        return

    hat_offsets = (0.0, 1.0, 2.0, 3.0)
    if state.drum_pattern == 2:
        hat_offsets = (0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5)
    for beat_offset in hat_offsets:
        add_humanized_note(track, bar, beat_offset, 0.12, 9, 42, 24 + state.drum_pattern * 6, rng, timing=0.015, velocity_spread=4)

    kick_pattern = (0.0, 2.0)
    snare_pattern = (3.0,)
    if state.drum_pattern == 2:
        kick_pattern = (0.0, 1.5, 2.75)
        snare_pattern = (2.0, 3.0)
    for beat_offset in kick_pattern:
        add_humanized_note(track, bar, beat_offset, 0.14, 9, 36, 34 + state.drum_pattern * 5, rng, timing=0.015, velocity_spread=4)
    for beat_offset in snare_pattern:
        add_humanized_note(track, bar, beat_offset, 0.14, 9, 38, 22 + state.drum_pattern * 4, rng, timing=0.015, velocity_spread=4)

    if bar % 8 == 7:
        add_humanized_note(track, bar, 3.5, 0.08, 9, 38, 34, rng, timing=0.01, velocity_spread=3)


def populate_tracks(
    spec: SongSpec,
    lead: Track | AudioTrack,
    comp: Track | AudioTrack,
    pad: Track | AudioTrack,
    bass: Track | AudioTrack,
    drums: Track | AudioTrack,
) -> int:
    progression = [spec.progression[i % len(spec.progression)] for i in range(spec.bars)]
    for bar in range(spec.bars):
        state = arrangement_state(spec, bar)
        rng = random.Random(stable_seed(spec.slug, bar))
        chord = progression[bar]
        add_pad(pad, spec, bar, chord, state, rng)
        add_comp(comp, spec, bar, chord, state, rng)
        add_bass(bass, spec, progression, bar, state, rng)
        if state.lead_active:
            add_lead(lead, spec, bar, state, rng)
        if spec.drums or state.section in {"verse_b", "finale"}:
            add_drums(drums, bar, state, rng)
    return spec.bars * BAR_TICKS + PPQ


def add_phrase_events(
    track: Track | AudioTrack,
    bar: int,
    channel: int,
    notes: list[tuple[float, float, int, int]],
    rng: random.Random,
    timing: float = 0.03,
    velocity_spread: int = 7,
) -> None:
    for offset, duration, note, velocity in notes:
        add_humanized_note(
            track,
            bar,
            offset,
            duration,
            channel,
            note,
            velocity,
            rng,
            timing=timing,
            velocity_spread=velocity_spread,
        )


def theme_comp_pattern(style: str) -> tuple[tuple[float, float, int], ...]:
    if style == "intro":
        return ((0.0, 0.6, 0), (0.75, 0.4, 1), (1.5, 0.55, 2), (2.25, 0.4, 1), (3.0, 0.65, 3))
    if style == "bridge":
        return ((0.0, 0.6, 0), (0.75, 0.45, 1), (1.5, 0.6, 2), (2.25, 0.45, 3), (3.0, 0.7, 1))
    if style == "lift":
        return ((0.0, 0.45, 0), (0.75, 0.35, 1), (1.5, 0.45, 2), (2.0, 0.45, 3), (2.75, 0.35, 1), (3.25, 0.45, 2))
    if style == "outro":
        return ((0.0, 0.8, 0), (1.0, 0.45, 1), (1.75, 0.45, 2), (2.5, 0.8, 3))
    return ((0.0, 0.5, 0), (0.75, 0.35, 1), (1.5, 0.45, 2), (2.25, 0.35, 1), (2.75, 0.45, 3), (3.35, 0.3, 2))


def theme_bass_pattern(style: str) -> tuple[tuple[float, float, str], ...]:
    if style == "intro":
        return ((0.0, 0.9, "root"), (1.0, 0.45, "fifth"), (2.0, 0.7, "octave"), (3.0, 0.45, "step"))
    if style == "bridge":
        return ((0.0, 0.9, "root"), (1.5, 0.6, "fifth"), (2.25, 0.6, "step"), (3.0, 0.7, "octave"))
    if style == "lift":
        return ((0.0, 0.8, "root"), (1.0, 0.5, "fifth"), (2.0, 0.8, "octave"), (3.0, 0.5, "step"))
    if style == "outro":
        return ((0.0, 1.2, "root"), (1.75, 0.8, "fifth"), (3.0, 0.6, "octave"))
    return ((0.0, 0.9, "root"), (1.0, 0.45, "fifth"), (2.0, 0.8, "octave"), (3.0, 0.45, "step"))


def resolve_theme_bass(current_bass: int, next_bass: int, role: str) -> int:
    if role == "root":
        return current_bass
    if role == "fifth":
        return current_bass + 7
    if role == "octave":
        return current_bass + 12
    return current_bass + (1 if next_bass >= current_bass else -1)


def schedule_theme_bar(
    comp: Track | AudioTrack,
    pad: Track | AudioTrack,
    bass: Track | AudioTrack,
    lead: Track | AudioTrack,
    counter: Track | AudioTrack,
    bar: int,
    chord: dict[str, object],
    style: str,
    lead_phrase: list[tuple[float, float, int, int]] | None,
    counter_phrase: list[tuple[float, float, int, int]] | None,
    next_bass: int,
) -> None:
    rng = random.Random(stable_seed(f"theme-{style}", bar))
    pad_notes = chord["pad"]
    comp_notes = chord["comp"]
    bass_root = int(chord["bass"])

    pad_segments = ((0.0, 4.0),)
    if style in {"bridge", "lift"}:
        pad_segments = ((0.0, 2.0), (2.0, 2.0))
    for seg_offset, seg_duration in pad_segments:
        for idx, note in enumerate(pad_notes):
            add_humanized_note(
                pad,
                bar,
                seg_offset + idx * 0.01,
                seg_duration - 0.06,
                2,
                int(note),
                46 if style == "intro" else 52,
                rng,
                timing=0.015,
                velocity_spread=4,
            )

    for offset, duration, idx in theme_comp_pattern(style):
        note = int(comp_notes[idx % len(comp_notes)])
        add_humanized_note(
            comp,
            bar,
            offset,
            duration,
            1,
            note,
            56 if style in {"bridge", "lift"} else 52,
            rng,
            timing=0.025,
            velocity_spread=6,
        )

    for offset, duration, role in theme_bass_pattern(style):
        note = resolve_theme_bass(bass_root, next_bass, role)
        add_humanized_note(
            bass,
            bar,
            offset,
            duration,
            3,
            note,
            58,
            rng,
            timing=0.02,
            velocity_spread=5,
        )

    if lead_phrase:
        add_phrase_events(lead, bar, 0, lead_phrase, rng, timing=0.035, velocity_spread=8)
    if counter_phrase:
        add_phrase_events(counter, bar, 4, counter_phrase, rng, timing=0.025, velocity_spread=6)


def schedule_title_theme_bar(
    comp: Track | AudioTrack,
    pad: Track | AudioTrack,
    bass: Track | AudioTrack,
    lead: Track | AudioTrack,
    bar: int,
    chord: dict[str, object],
    style: str,
    lead_phrase: list[tuple[float, float, int, int]] | None,
    next_bass: int,
) -> None:
    rng = random.Random(stable_seed(f"title-theme-{style}", bar))
    pad_notes = [int(note) for note in chord["pad"]]
    comp_notes = [int(note) for note in chord["comp"]]
    bass_root = int(chord["bass"])

    pad_segments: tuple[tuple[float, float], ...]
    if style == "intro":
        pad_segments = ((0.0, 4.0),)
    elif style == "outro":
        pad_segments = ((0.0, 2.0),)
    else:
        pad_segments = ()
    for seg_offset, seg_duration in pad_segments:
        for idx, note in enumerate(pad_notes[:3]):
            add_humanized_note(
                pad,
                bar,
                seg_offset + idx * 0.01,
                seg_duration - 0.08,
                2,
                note,
                34 if style == "intro" else 30,
                rng,
                timing=0.012,
                velocity_spread=3,
            )

    comp_hits = ((0.0, 0.65), (2.0, 0.55))
    if style == "bridge":
        comp_hits = ((0.0, 0.55), (1.5, 0.45), (3.0, 0.45))
    if style == "outro":
        comp_hits = ((0.0, 0.8), (2.25, 0.7))
    for offset, duration in comp_hits:
        for idx, note in enumerate(comp_notes[:3]):
            add_humanized_note(
                comp,
                bar,
                offset + idx * 0.01,
                duration,
                1,
                note,
                40 if style == "bridge" else 36,
                rng,
                timing=0.018,
                velocity_spread=4,
            )

    bass_pattern = ((0.0, 1.0, "root"), (2.0, 0.75, "fifth"))
    if style == "bridge":
        bass_pattern = ((0.0, 1.0, "root"), (1.5, 0.6, "fifth"), (3.0, 0.6, "step"))
    if style == "outro":
        bass_pattern = ((0.0, 1.5, "root"), (2.25, 0.9, "octave"))
    for offset, duration, role in bass_pattern:
        note = resolve_theme_bass(bass_root, next_bass, role)
        add_humanized_note(
            bass,
            bar,
            offset,
            duration,
            3,
            note,
            54,
            rng,
            timing=0.016,
            velocity_spread=4,
        )

    if lead_phrase:
        add_phrase_events(lead, bar, 0, lead_phrase, rng, timing=0.008, velocity_spread=3)


def build_civic_sunrise_theme(
    lead: Track | AudioTrack,
    comp: Track | AudioTrack,
    pad: Track | AudioTrack,
    bass: Track | AudioTrack,
    counter: Track | AudioTrack,
) -> int:
    intro_chords = [
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
    ]
    a_chords = [
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 40, "pad": [52, 59, 62, 67], "comp": [59, 62, 67, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
    ]
    bridge_chords = [
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 35, "pad": [47, 54, 57, 62], "comp": [59, 62, 66, 71]},
        {"bass": 40, "pad": [52, 59, 62, 67], "comp": [59, 62, 67, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
    ]
    b_chords = [
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 35, "pad": [47, 54, 57, 62], "comp": [59, 62, 66, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 40, "pad": [52, 59, 62, 67], "comp": [59, 62, 67, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
    ]
    reprise_chords = [
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
        {"bass": 40, "pad": [52, 59, 62, 67], "comp": [59, 62, 67, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 69]},
        {"bass": 38, "pad": [50, 57, 59, 64], "comp": [57, 59, 62, 66]},
    ]

    intro_lead = [
        [],
        [],
        [(2.0, 1.0, 74, 68), (3.0, 1.0, 79, 72)],
        [(0.0, 1.0, 81, 74), (1.0, 1.0, 79, 72), (2.0, 1.0, 76, 70), (3.0, 1.0, 74, 68)],
        [(0.0, 1.0, 74, 74), (1.0, 1.0, 79, 78), (2.0, 1.0, 81, 80), (3.0, 1.0, 79, 78)],
        [(0.0, 1.0, 78, 76), (1.0, 1.0, 76, 74), (2.0, 1.0, 74, 72), (3.0, 1.0, 71, 70)],
        [(0.0, 1.0, 72, 72), (1.0, 1.0, 76, 74), (2.0, 1.0, 79, 78), (3.0, 1.0, 81, 80)],
        [(0.0, 1.0, 79, 76), (1.0, 1.0, 76, 74), (2.0, 1.0, 74, 72), (3.0, 1.0, 72, 70)],
    ]
    a_lead = [
        [(0.0, 1.0, 74, 76), (1.0, 1.0, 79, 80), (2.0, 1.0, 81, 82), (3.0, 1.0, 79, 80)],
        [(0.0, 1.0, 78, 76), (1.0, 1.0, 76, 74), (2.0, 1.0, 74, 72), (3.0, 1.0, 71, 72)],
        [(0.0, 1.0, 72, 74), (1.0, 1.0, 76, 76), (2.0, 1.0, 79, 80), (3.0, 1.0, 81, 82)],
        [(0.0, 1.0, 79, 78), (1.0, 1.0, 76, 76), (2.0, 1.0, 74, 74), (3.0, 1.0, 72, 72)],
        [(0.0, 1.0, 74, 76), (1.0, 1.0, 79, 80), (2.0, 1.0, 81, 82), (3.0, 1.0, 83, 84)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 79, 78), (2.0, 1.0, 76, 76), (3.0, 1.0, 74, 74)],
        [(0.0, 1.0, 72, 74), (1.0, 1.0, 76, 76), (2.0, 1.0, 79, 78), (3.0, 1.0, 81, 80)],
        [(0.0, 1.0, 79, 76), (1.0, 1.0, 76, 74), (2.0, 1.0, 74, 72), (3.0, 1.25, 71, 70)],
    ]
    a_variation = [
        [(0.0, 1.0, 79, 80), (1.0, 1.0, 81, 82), (2.0, 1.0, 83, 84), (3.0, 1.0, 81, 82)],
        [(0.0, 1.0, 78, 76), (1.0, 1.0, 76, 74), (2.0, 1.0, 74, 72), (3.0, 1.0, 71, 72)],
        [(0.0, 1.0, 76, 76), (1.0, 1.0, 79, 78), (2.0, 1.0, 81, 82), (3.0, 1.0, 83, 84)],
        [(0.0, 1.0, 84, 82), (1.0, 1.0, 81, 80), (2.0, 1.0, 79, 78), (3.0, 1.0, 76, 76)],
        [(0.0, 1.0, 79, 80), (1.0, 1.0, 81, 82), (2.0, 1.0, 83, 84), (3.0, 1.0, 84, 86)],
        [(0.0, 1.0, 83, 82), (1.0, 1.0, 81, 80), (2.0, 1.0, 79, 78), (3.0, 1.0, 76, 76)],
        [(0.0, 1.0, 76, 76), (1.0, 1.0, 79, 78), (2.0, 1.0, 81, 80), (3.0, 1.0, 83, 82)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 79, 78), (2.0, 1.0, 76, 76), (3.0, 1.25, 74, 74)],
    ]
    bridge_lead = [
        [(0.0, 1.5, 72, 72), (2.0, 1.5, 76, 74)],
        [(0.0, 1.5, 74, 74), (2.0, 1.5, 78, 76)],
        [(0.0, 1.5, 76, 76), (2.0, 1.5, 79, 78)],
        [(0.0, 1.5, 74, 74), (2.0, 1.5, 76, 76)],
        [(0.0, 1.5, 72, 72), (2.0, 1.5, 76, 74)],
        [(0.0, 1.5, 71, 72), (2.0, 1.5, 74, 74)],
        [(0.0, 1.5, 72, 74), (2.0, 1.5, 79, 78)],
        [(0.0, 1.5, 76, 76), (2.0, 1.5, 74, 74)],
    ]
    b_lead = [
        [(0.0, 1.0, 79, 82), (1.0, 1.0, 83, 84), (2.0, 1.0, 86, 86), (3.0, 1.0, 83, 84)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 83, 82), (2.0, 1.0, 84, 84), (3.0, 1.0, 81, 80)],
        [(0.0, 1.0, 79, 78), (1.0, 1.0, 81, 80), (2.0, 1.0, 83, 82), (3.0, 1.0, 84, 84)],
        [(0.0, 1.0, 83, 82), (1.0, 1.0, 81, 80), (2.0, 1.0, 79, 78), (3.0, 1.0, 76, 76)],
        [(0.0, 1.0, 79, 82), (1.0, 1.0, 83, 84), (2.0, 1.0, 86, 88), (3.0, 1.0, 88, 90)],
        [(0.0, 1.0, 86, 86), (1.0, 1.0, 83, 84), (2.0, 1.0, 81, 82), (3.0, 1.0, 79, 80)],
        [(0.0, 1.0, 76, 76), (1.0, 1.0, 79, 78), (2.0, 1.0, 81, 80), (3.0, 1.0, 83, 82)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 79, 78), (2.0, 1.0, 76, 76), (3.0, 1.25, 74, 74)],
    ]
    b_variation = [
        [(0.0, 1.0, 83, 84), (1.0, 1.0, 86, 86), (2.0, 1.0, 88, 88), (3.0, 1.0, 86, 86)],
        [(0.0, 1.0, 84, 84), (1.0, 1.0, 83, 82), (2.0, 1.0, 81, 80), (3.0, 1.0, 79, 80)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 83, 82), (2.0, 1.0, 84, 84), (3.0, 1.0, 86, 86)],
        [(0.0, 1.0, 88, 88), (1.0, 1.0, 86, 86), (2.0, 1.0, 83, 84), (3.0, 1.0, 81, 82)],
        [(0.0, 1.0, 83, 84), (1.0, 1.0, 86, 86), (2.0, 1.0, 88, 88), (3.0, 1.0, 91, 90)],
        [(0.0, 1.0, 88, 88), (1.0, 1.0, 86, 86), (2.0, 1.0, 83, 84), (3.0, 1.0, 81, 82)],
        [(0.0, 1.0, 79, 78), (1.0, 1.0, 81, 80), (2.0, 1.0, 83, 82), (3.0, 1.0, 84, 84)],
        [(0.0, 1.0, 83, 82), (1.0, 1.0, 81, 80), (2.0, 1.0, 79, 78), (3.0, 1.25, 76, 76)],
    ]
    reprise_lead = [
        [(0.0, 1.0, 79, 80), (1.0, 1.0, 81, 82), (2.0, 1.0, 83, 84), (3.0, 1.0, 81, 82)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 79, 78), (2.0, 1.0, 76, 76), (3.0, 1.0, 74, 74)],
        [(0.0, 1.0, 76, 76), (1.0, 1.0, 79, 78), (2.0, 1.0, 81, 80), (3.0, 1.0, 83, 82)],
        [(0.0, 1.0, 84, 82), (1.0, 1.0, 81, 80), (2.0, 1.0, 79, 78), (3.0, 1.0, 76, 76)],
        [(0.0, 1.0, 79, 80), (1.0, 1.0, 81, 82), (2.0, 1.0, 83, 84), (3.0, 1.0, 81, 82)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 79, 78), (2.0, 1.0, 76, 76), (3.0, 1.0, 74, 74)],
        [(0.0, 1.0, 76, 76), (1.0, 1.0, 79, 78), (2.0, 1.0, 81, 80), (3.0, 1.0, 83, 82)],
        [(0.0, 1.0, 81, 80), (1.0, 1.0, 79, 78), (2.0, 1.0, 76, 76), (3.0, 1.25, 74, 74)],
    ]
    outro_lead = [
        [(0.0, 1.0, 79, 72), (1.0, 1.0, 81, 74), (2.0, 1.0, 79, 72)],
        [(0.0, 1.0, 83, 76), (1.0, 1.0, 81, 74), (2.0, 1.0, 79, 72)],
        [(0.0, 1.0, 79, 72), (1.0, 1.0, 83, 76), (2.0, 1.0, 81, 74)],
        [(0.0, 3.0, 79, 70)],
        [],
        [],
        [],
        [],
    ]

    sections = [
        ("intro", intro_chords[:4], intro_lead[:4], [None] * 4),
        ("a", a_chords, a_lead, [None] * 8),
        ("lift", b_chords, b_lead, [None] * 8),
        ("bridge", bridge_chords, bridge_lead, [None] * 8),
        ("lift", b_chords, b_variation, [None] * 8),
        ("outro", reprise_chords[:4], outro_lead[:4], [None] * 4),
    ]

    bar_cursor = 0
    all_chords: list[dict[str, object]] = []
    for _, chords, _, _ in sections:
        all_chords.extend(chords)
    for style, chords, lead_bars, counter_bars in sections:
        for idx, chord in enumerate(chords):
            absolute_bar = bar_cursor + idx
            next_chord = all_chords[(absolute_bar + 1) % len(all_chords)]
            schedule_title_theme_bar(
                comp,
                pad,
                bass,
                lead,
                absolute_bar,
                chord,
                style,
                lead_bars[idx] if idx < len(lead_bars) else None,
                int(next_chord["bass"]),
            )
        bar_cursor += len(chords)

    return bar_cursor * BAR_TICKS + PPQ


def build_riverfront_reflections_light(
    lead: Track | AudioTrack,
    comp: Track | AudioTrack,
    pad: Track | AudioTrack,
    bass: Track | AudioTrack,
    counter: Track | AudioTrack,
) -> int:
    intro_chords = [
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 64, 67, 71]},
        {"bass": 45, "pad": [45, 52, 55, 59], "comp": [60, 64, 67, 71]},
        {"bass": 41, "pad": [41, 48, 52, 55], "comp": [57, 60, 64, 67]},
        {"bass": 38, "pad": [50, 57, 60, 64], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 53, 60], "comp": [59, 62, 67, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 64, 67, 71]},
    ]
    a_chords = [
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 64, 67, 71]},
        {"bass": 45, "pad": [45, 52, 55, 59], "comp": [60, 64, 67, 71]},
        {"bass": 41, "pad": [41, 48, 52, 55], "comp": [57, 60, 64, 67]},
        {"bass": 38, "pad": [50, 57, 60, 64], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 53, 60], "comp": [59, 62, 67, 71]},
        {"bass": 40, "pad": [40, 47, 52, 55], "comp": [57, 60, 64, 67]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 64, 67, 71]},
    ]
    bridge_chords = [
        {"bass": 45, "pad": [45, 52, 57, 60], "comp": [60, 64, 69, 72]},
        {"bass": 47, "pad": [47, 54, 57, 62], "comp": [62, 66, 69, 74]},
        {"bass": 48, "pad": [48, 55, 59, 64], "comp": [64, 67, 71, 76]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 64, 67, 71]},
        {"bass": 45, "pad": [45, 52, 55, 60], "comp": [60, 64, 67, 72]},
        {"bass": 41, "pad": [41, 48, 52, 57], "comp": [57, 60, 64, 69]},
        {"bass": 38, "pad": [50, 57, 60, 64], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 71]},
    ]
    b_chords = [
        {"bass": 36, "pad": [48, 55, 59, 64], "comp": [64, 67, 71, 76]},
        {"bass": 40, "pad": [40, 47, 52, 55], "comp": [59, 64, 67, 71]},
        {"bass": 41, "pad": [41, 48, 52, 57], "comp": [60, 64, 69, 72]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [62, 67, 71, 74]},
        {"bass": 45, "pad": [45, 52, 55, 60], "comp": [64, 67, 72, 76]},
        {"bass": 47, "pad": [47, 54, 57, 62], "comp": [66, 69, 74, 78]},
        {"bass": 38, "pad": [50, 57, 60, 64], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 62, 67, 71]},
    ]
    outro_chords = [
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 55, 59], "comp": [59, 64, 67, 71]},
        {"bass": 45, "pad": [45, 52, 55, 59], "comp": [60, 64, 67, 71]},
        {"bass": 41, "pad": [41, 48, 52, 55], "comp": [57, 60, 64, 67]},
        {"bass": 38, "pad": [50, 57, 60, 64], "comp": [60, 64, 67, 71]},
        {"bass": 43, "pad": [43, 50, 53, 60], "comp": [59, 62, 67, 71]},
        {"bass": 36, "pad": [48, 55, 59, 62], "comp": [60, 64, 67, 71]},
        {"bass": 36, "pad": [48, 55, 60, 64], "comp": [60, 64, 67, 72]},
    ]

    intro_lead = [
        [],
        [],
        [(1.0, 0.8, 76, 66), (2.25, 0.8, 79, 68)],
        [(0.5, 0.8, 77, 64), (1.75, 0.8, 74, 62)],
        [(0.0, 0.6, 72, 62), (0.85, 0.5, 74, 64), (1.6, 0.7, 76, 66), (2.7, 0.8, 79, 68)],
        [(0.0, 0.8, 81, 70), (1.25, 0.7, 79, 68), (2.4, 0.8, 76, 66)],
        [(0.0, 0.6, 74, 66), (0.85, 0.5, 76, 68), (1.6, 0.7, 79, 70), (2.7, 0.8, 81, 72)],
        [(0.0, 0.7, 79, 68), (1.0, 0.6, 76, 66), (2.0, 1.0, 72, 64)],
    ]
    a_lead = [
        [(0.0, 0.55, 76, 72), (0.85, 0.45, 79, 70), (1.55, 0.55, 81, 72), (2.4, 0.8, 84, 74)],
        [(0.0, 0.55, 83, 72), (0.85, 0.45, 81, 70), (1.55, 0.55, 79, 68), (2.4, 0.8, 76, 68)],
        [(0.0, 0.55, 77, 70), (0.85, 0.45, 79, 68), (1.55, 0.55, 81, 70), (2.4, 0.8, 84, 72)],
        [(0.0, 0.55, 81, 70), (0.85, 0.45, 79, 68), (1.55, 0.55, 77, 66), (2.4, 0.8, 74, 66)],
        [(0.0, 0.55, 76, 68), (0.85, 0.45, 77, 66), (1.55, 0.55, 79, 68), (2.4, 0.8, 81, 70)],
        [(0.0, 0.55, 83, 72), (0.85, 0.45, 81, 70), (1.55, 0.55, 79, 68), (2.4, 0.8, 76, 68)],
        [(0.0, 0.55, 74, 66), (0.85, 0.45, 76, 68), (1.55, 0.55, 77, 68), (2.4, 0.8, 79, 70)],
        [(0.0, 0.55, 81, 72), (0.85, 0.45, 79, 70), (1.55, 0.55, 76, 68), (2.4, 1.0, 72, 66)],
    ]
    a_variation = [
        [(0.0, 0.45, 84, 78), (0.7, 0.35, 83, 72), (1.3, 0.45, 81, 70), (2.0, 0.55, 79, 70), (2.85, 0.55, 81, 72)],
        [(0.0, 0.45, 83, 72), (0.7, 0.35, 81, 70), (1.3, 0.45, 79, 68), (2.0, 0.55, 76, 68), (2.85, 0.55, 79, 70)],
        [(0.0, 0.45, 81, 72), (0.7, 0.35, 83, 72), (1.3, 0.45, 84, 74), (2.0, 0.55, 86, 76), (2.85, 0.55, 84, 74)],
        [(0.0, 0.45, 81, 70), (0.7, 0.35, 79, 68), (1.3, 0.45, 77, 66), (2.0, 0.55, 74, 66), (2.85, 0.55, 76, 68)],
        [(0.0, 0.45, 79, 68), (0.7, 0.35, 81, 70), (1.3, 0.45, 83, 72), (2.0, 0.55, 84, 74), (2.85, 0.55, 83, 72)],
        [(0.0, 0.45, 81, 72), (0.7, 0.35, 79, 70), (1.3, 0.45, 76, 68), (2.0, 0.55, 74, 66), (2.85, 0.55, 76, 68)],
        [(0.0, 0.45, 77, 68), (0.7, 0.35, 79, 70), (1.3, 0.45, 81, 72), (2.0, 0.55, 83, 72), (2.85, 0.55, 81, 70)],
        [(0.0, 0.45, 79, 70), (0.7, 0.35, 77, 68), (1.3, 0.45, 76, 68), (2.0, 0.55, 74, 66), (2.85, 1.0, 72, 64)],
    ]
    bridge_lead = [
        [(0.0, 0.8, 72, 66), (1.25, 0.8, 76, 68), (2.5, 0.8, 79, 70)],
        [(0.0, 0.8, 74, 68), (1.25, 0.8, 78, 70), (2.5, 0.8, 81, 72)],
        [(0.0, 0.8, 76, 70), (1.25, 0.8, 79, 72), (2.5, 0.8, 83, 74)],
        [(0.0, 0.8, 74, 68), (1.25, 0.8, 79, 70), (2.5, 0.8, 81, 72)],
        [(0.0, 0.8, 72, 66), (1.25, 0.8, 76, 68), (2.5, 0.8, 79, 70)],
        [(0.0, 0.8, 71, 66), (1.25, 0.8, 74, 68), (2.5, 0.8, 77, 70)],
        [(0.0, 0.8, 72, 68), (1.25, 0.8, 76, 70), (2.5, 0.8, 81, 72)],
        [(0.0, 0.8, 79, 72), (1.25, 0.8, 76, 70), (2.5, 0.8, 74, 68)],
    ]
    b_lead = [
        [(0.0, 0.45, 84, 78), (0.7, 0.35, 86, 80), (1.3, 0.45, 88, 82), (2.0, 0.55, 86, 80), (2.85, 0.55, 84, 78)],
        [(0.0, 0.45, 83, 76), (0.7, 0.35, 84, 78), (1.3, 0.45, 86, 80), (2.0, 0.55, 84, 78), (2.85, 0.55, 83, 76)],
        [(0.0, 0.45, 81, 74), (0.7, 0.35, 83, 76), (1.3, 0.45, 84, 78), (2.0, 0.55, 86, 80), (2.85, 0.55, 88, 82)],
        [(0.0, 0.45, 86, 80), (0.7, 0.35, 84, 78), (1.3, 0.45, 83, 76), (2.0, 0.55, 81, 74), (2.85, 0.55, 79, 72)],
        [(0.0, 0.45, 84, 78), (0.7, 0.35, 86, 80), (1.3, 0.45, 88, 82), (2.0, 0.55, 91, 84), (2.85, 0.55, 88, 82)],
        [(0.0, 0.45, 86, 80), (0.7, 0.35, 88, 82), (1.3, 0.45, 89, 84), (2.0, 0.55, 88, 82), (2.85, 0.55, 86, 80)],
        [(0.0, 0.45, 81, 74), (0.7, 0.35, 83, 76), (1.3, 0.45, 84, 78), (2.0, 0.55, 86, 80), (2.85, 0.55, 83, 76)],
        [(0.0, 0.45, 84, 78), (0.7, 0.35, 81, 74), (1.3, 0.45, 79, 72), (2.0, 0.55, 76, 70), (2.85, 1.0, 74, 68)],
    ]
    outro_lead = [
        [(0.0, 0.7, 79, 68), (1.25, 0.7, 76, 66), (2.5, 0.8, 72, 64)],
        [(0.0, 0.7, 79, 68), (1.25, 0.7, 76, 66), (2.5, 0.8, 74, 66)],
        [(0.0, 0.7, 81, 70), (1.25, 0.7, 79, 68), (2.5, 0.8, 76, 66)],
        [(0.0, 0.7, 77, 66), (1.25, 0.7, 74, 64), (2.5, 0.8, 72, 62)],
        [(0.0, 0.7, 76, 66), (1.25, 0.7, 79, 68), (2.5, 0.8, 81, 70)],
        [(0.0, 0.7, 79, 68), (1.25, 0.7, 76, 66), (2.5, 0.8, 74, 64)],
        [(0.0, 0.9, 72, 64), (1.5, 0.9, 76, 66)],
        [(0.0, 2.6, 79, 62)],
    ]

    bridge_counter = [
        [(0.5, 1.2, 67, 52), (2.2, 1.0, 72, 54)],
        [(0.5, 1.2, 69, 54), (2.2, 1.0, 74, 56)],
        [(0.5, 1.2, 71, 56), (2.2, 1.0, 76, 58)],
        [(0.5, 1.2, 69, 54), (2.2, 1.0, 74, 56)],
        [(0.5, 1.2, 67, 52), (2.2, 1.0, 72, 54)],
        [(0.5, 1.2, 65, 50), (2.2, 1.0, 71, 52)],
        [(0.5, 1.2, 67, 52), (2.2, 1.0, 72, 54)],
        [(1.0, 1.6, 71, 52)],
    ]
    b_counter = [
        [(0.5, 1.2, 76, 56), (2.2, 1.0, 79, 58)],
        [(0.5, 1.2, 74, 54), (2.2, 1.0, 78, 56)],
        [(0.5, 1.2, 76, 56), (2.2, 1.0, 81, 58)],
        [(0.5, 1.2, 74, 54), (2.2, 1.0, 79, 56)],
        [(0.5, 1.2, 76, 58), (2.2, 1.0, 81, 60)],
        [(0.5, 1.2, 78, 58), (2.2, 1.0, 83, 60)],
        [(0.5, 1.2, 76, 56), (2.2, 1.0, 79, 58)],
        [(1.0, 1.6, 78, 56)],
    ]

    sections = [
        ("intro", intro_chords, intro_lead, [None] * 8),
        ("a", a_chords, a_lead, [None] * 8),
        ("a", a_chords, a_variation, [None] * 8),
        ("bridge", bridge_chords, bridge_lead, bridge_counter),
        ("lift", b_chords, b_lead, b_counter),
        ("lift", b_chords, b_lead, b_counter),
        ("a", a_chords, a_variation, [None] * 8),
        ("outro", outro_chords, outro_lead, [None] * 8),
    ]

    bar_cursor = 0
    all_chords: list[dict[str, object]] = []
    for _, chords, _, _ in sections:
        all_chords.extend(chords)
    for style, chords, lead_bars, counter_bars in sections:
        for idx, chord in enumerate(chords):
            absolute_bar = bar_cursor + idx
            next_chord = all_chords[(absolute_bar + 1) % len(all_chords)]
            schedule_theme_bar(
                comp,
                pad,
                bass,
                lead,
                counter,
                absolute_bar,
                chord,
                style,
                lead_bars[idx] if idx < len(lead_bars) else None,
                counter_bars[idx] if idx < len(counter_bars) else None,
                int(next_chord["bass"]),
            )
        bar_cursor += len(chords)

    return bar_cursor * BAR_TICKS + PPQ


def build_metropolitan_mists(
    lead: Track | AudioTrack,
    comp: Track | AudioTrack,
    pad: Track | AudioTrack,
    bass: Track | AudioTrack,
    counter: Track | AudioTrack,
) -> int:
    # --- HARMONY (Jazz-Fusion Voicings) ---
    eb_maj9 = {"bass": 39, "pad": [51, 58, 62, 65], "comp": [62, 65, 70, 74]}
    ab_13   = {"bass": 44, "pad": [56, 60, 64, 66], "comp": [64, 66, 68, 72]}
    f_m9    = {"bass": 41, "pad": [53, 60, 63, 65], "comp": [60, 63, 68, 72]}
    bb_13su = {"bass": 46, "pad": [58, 63, 66, 68], "comp": [63, 66, 70, 73]}
    c_m11   = {"bass": 48, "pad": [51, 55, 58, 62], "comp": [63, 65, 70, 72]}

    prog = [eb_maj9, ab_13, eb_maj9, bb_13su, eb_maj9, ab_13, c_m11, f_m9]
    bridge_prog = [f_m9, bb_13su, eb_maj9, c_m11, f_m9, bb_13su, ab_13, bb_13su]

    def add_syncopated_comp(bar, chords):
        rng = random.Random(stable_seed("mists-comp", bar))
        # Rhodes "Stabs" landing on the off-beats
        stabs = [(0.0, 0.2, 85), (1.5, 0.4, 75), (2.25, 0.2, 90), (3.5, 0.3, 70)]
        chord = chords[bar % len(chords)]
        for offset, dur, vel in stabs:
            for note in chord["comp"]:
                add_humanized_note(comp, bar, offset, dur, 1, note, vel, rng, timing=0.01)

    def add_walking_bass(bar, chords):
        rng = random.Random(stable_seed("mists-bass", bar))
        root = chords[bar % len(chords)]["bass"]
        next_root = chords[(bar + 1) % len(chords)]["bass"]
        # Classic syncopated fusion bass line
        pattern = [
            (0.0, 0.8, root, 95),
            (1.0, 0.4, root + 7, 85),
            (1.5, 0.6, root + 12, 90),
            (2.5, 0.4, root + 10, 80),
            (3.0, 0.5, root + 5, 85),
            (3.5, 0.3, next_root - 1, 75) # Chromatic approach
        ]
        for offset, dur, note, vel in pattern:
            add_humanized_note(bass, bar, offset, dur, 3, note, vel, rng, timing=0.015)

    def add_expressive_lead(bar, phrase_type):
        rng = random.Random(stable_seed("mists-lead", bar))
        # Use triplets and syncopation for the flute
        if phrase_type == "call":
            notes = [
                (0.0, 1.2, 75, 60),  # Soft start
                (1.33, 0.3, 79, 85), # Pushed triplet
                (1.66, 0.3, 82, 95),
                (2.0, 1.5, 84, 105)  # Peak velocity
            ]
        elif phrase_type == "resp":
            notes = [
                (0.0, 0.8, 82, 90),
                (1.0, 0.4, 80, 80),
                (1.5, 1.5, 79, 70)   # Tail-off
            ]
        else:
            notes = []
            
        for offset, dur, note, vel in notes:
            add_humanized_note(lead, bar, offset, dur, 0, note, vel, rng, timing=0.005)

    def add_fusion_drums(bar):
        rng = random.Random(stable_seed("mists-drums", bar))
        # Soft shaker / hi-hat groove
        for i in range(8):
            add_humanized_note(counter, bar, i*0.5, 0.1, 9, 42, 35 if i%2==0 else 20, rng)
        # Pushed kick/snare
        if bar % 2 == 0:
            add_humanized_note(counter, bar, 0.0, 0.2, 9, 36, 45, rng)
            add_humanized_note(counter, bar, 2.5, 0.2, 9, 38, 40, rng)

    # Build sequence
    for bar in range(64):
        if bar < 8: # Intro
            chords = prog
            add_fusion_drums(bar)
        elif bar < 24: # A Section
            chords = prog
            add_syncopated_comp(bar, chords)
            add_walking_bass(bar, chords)
            add_fusion_drums(bar)
            add_expressive_lead(bar, "call" if bar % 4 == 0 else "resp" if bar % 4 == 2 else "none")
        elif bar < 40: # B Section (Bridge)
            chords = bridge_prog
            add_syncopated_comp(bar, chords)
            add_walking_bass(bar, chords)
            add_fusion_drums(bar)
            # More active flute in bridge
            if bar % 2 == 0:
                add_humanized_note(lead, bar, 0.5, 0.5, 0, 87, 95, random.Random(bar))
                add_humanized_note(lead, bar, 1.5, 1.5, 0, 86, 80, random.Random(bar))
        else: # Outro
            chords = prog
            add_fusion_drums(bar)

        # Always add pad for atmosphere
        chord = chords[bar % len(chords)]
        for note in chord["pad"]:
            add_humanized_note(pad, bar, 0.0, 4.0, 2, note, 45, random.Random(bar), timing=0.02)

    return 64 * BAR_TICKS + PPQ


def make_song(spec: SongSpec) -> bytes:
    meta = Track(spec.title)
    meta.tempo(0, spec.bpm)
    meta.time_signature(0, 4, 4)
    meta.meta(0, 0x01, f"{spec.title} - original soundtrack piece".encode("ascii"))

    lead = Track("Lead")
    comp = Track("Comp")
    pad = Track("Pad")
    bass = Track("Bass")
    counter = Track("Counter")
    drums = Track("Drums")

    for track, channel, program, volume, pan in (
        (lead, 0, 0 if spec.slug == "01_civic_sunrise_theme" else 11 if spec.slug == "02_riverfront_reflections" else spec.lead_program, 96 if spec.slug == "01_civic_sunrise_theme" else 92, 74),
        (comp, 1, spec.comp_program, 70 if spec.slug == "01_civic_sunrise_theme" else 84, 56),
        (pad, 2, 88 if spec.slug == "01_civic_sunrise_theme" else 48 if spec.slug == "02_riverfront_reflections" else spec.pad_program, 58 if spec.slug == "01_civic_sunrise_theme" else 72, 82),
        (bass, 3, spec.bass_program, 78, 58),
        (counter, 4, 9 if spec.slug == "01_civic_sunrise_theme" else 73 if spec.slug == "02_riverfront_reflections" else spec.lead_program, 36 if spec.slug == "01_civic_sunrise_theme" else 68, 46),
    ):
        track.program(0, channel, program)
        track.control(0, channel, 7, volume)
        track.control(0, channel, 10, pan)
        track.control(0, channel, 91, 42)

    drums.control(0, 9, 7, 44)
    drums.control(0, 9, 91, 24)
    end_tick = (
        build_civic_sunrise_theme(lead, comp, pad, bass, counter)
        if spec.slug == "01_civic_sunrise_theme"
        else build_riverfront_reflections_light(lead, comp, pad, bass, counter)
        if spec.slug == "02_riverfront_reflections"
        else build_metropolitan_mists(lead, comp, pad, bass, counter)
        if spec.slug == "11_metropolitan_mists"
        else populate_tracks(spec, lead, comp, pad, bass, drums)
    )
    tracks = [meta, lead, comp, pad, bass]
    if counter.events:
        tracks.append(counter)
    if drums.events:
        tracks.append(drums)

    body = bytearray()
    body.extend(b"MThd")
    body.extend(struct.pack(">IHHH", 6, 1, len(tracks), PPQ))
    for track in tracks:
        body.extend(track.render(end_tick))
    return bytes(body)


def arrange_song(spec: SongSpec) -> tuple[list[AudioTrack], int]:
    lead = AudioTrack("Lead", "lead", 0)
    comp = AudioTrack("Comp", "comp", 1)
    pad = AudioTrack("Pad", "pad", 2)
    bass = AudioTrack("Bass", "bass", 3)
    counter = AudioTrack("Counter", "counter", 4)
    drums = AudioTrack("Drums", "drums", 9)

    for track, channel, program, volume, pan in (
        (lead, 0, 0 if spec.slug == "01_civic_sunrise_theme" else 11 if spec.slug == "02_riverfront_reflections" else spec.lead_program, 96 if spec.slug == "01_civic_sunrise_theme" else 92, 74),
        (comp, 1, spec.comp_program, 70 if spec.slug == "01_civic_sunrise_theme" else 84, 56),
        (pad, 2, 88 if spec.slug == "01_civic_sunrise_theme" else 48 if spec.slug == "02_riverfront_reflections" else spec.pad_program, 58 if spec.slug == "01_civic_sunrise_theme" else 72, 82),
        (bass, 3, spec.bass_program, 78, 58),
        (counter, 4, 9 if spec.slug == "01_civic_sunrise_theme" else 73 if spec.slug == "02_riverfront_reflections" else spec.lead_program, 36 if spec.slug == "01_civic_sunrise_theme" else 68, 46),
    ):
        track.program(0, channel, program)
        track.control(0, channel, 7, volume)
        track.control(0, channel, 10, pan)
        track.control(0, channel, 91, 42)

    drums.control(0, 9, 7, 44)
    drums.control(0, 9, 91, 24)
    end_tick = (
        build_civic_sunrise_theme(lead, comp, pad, bass, counter)
        if spec.slug == "01_civic_sunrise_theme"
        else build_riverfront_reflections_light(lead, comp, pad, bass, counter)
        if spec.slug == "02_riverfront_reflections"
        else build_metropolitan_mists(lead, comp, pad, bass, counter)
        if spec.slug == "11_metropolitan_mists"
        else populate_tracks(spec, lead, comp, pad, bass, drums)
    )
    tracks = [lead, comp, pad, bass]
    if counter.notes:
        tracks.append(counter)
    if drums.notes:
        tracks.append(drums)
    return tracks, end_tick


def midi_note_to_freq(note: int) -> float:
    return 440.0 * (2.0 ** ((note - 69) / 12.0))


def tick_to_seconds(spec: SongSpec, tick: int) -> float:
    return tick * 60.0 / (spec.bpm * PPQ)


def envelope_value(index: int, frames: int, attack_frames: int, release_frames: int) -> float:
    if frames <= 0:
        return 0.0
    if attack_frames > 0 and index < attack_frames:
        return index / attack_frames
    if release_frames > 0 and index >= frames - release_frames:
        remaining = frames - index
        return max(0.0, remaining / release_frames)
    return 1.0


def equal_power_pan(pan_value: int) -> tuple[float, float]:
    pan = max(0.0, min(1.0, pan_value / 127.0))
    return math.cos(pan * math.pi * 0.5), math.sin(pan * math.pi * 0.5)


def table_sample(table: list[float], phase: float) -> float:
    return table[int(phase) & TABLE_MASK]


def synth_voice(kind: str, freq: float, phase: float, mod_phase: float) -> float:
    if kind == "counter":
        return (
            0.42 * table_sample(SINE_TABLE, phase)
            + 0.18 * table_sample(TRI_TABLE, phase * 2.0)
            + 0.10 * table_sample(SINE_TABLE, phase * 0.5 + mod_phase * 0.08)
        )
    if kind == "lead":
        return (
            0.46 * table_sample(SINE_TABLE, phase + table_sample(SINE_TABLE, mod_phase) * 8.0)
            + 0.21 * table_sample(TRI_TABLE, phase * 1.003 + mod_phase * 0.22)
            + 0.10 * table_sample(SAW_TABLE, phase * 2.0)
            + 0.06 * table_sample(SINE_TABLE, phase * 0.5)
        )
    if kind == "comp":
        return (
            0.34 * table_sample(SINE_TABLE, phase)
            + 0.28 * table_sample(SINE_TABLE, phase * 2.0)
            + 0.18 * table_sample(TRI_TABLE, phase * 3.0)
            + 0.08 * table_sample(NOISE_TABLE, mod_phase * 0.5)
        )
    if kind == "pad":
        return (
            0.34 * table_sample(SINE_TABLE, phase * 0.996)
            + 0.34 * table_sample(SINE_TABLE, phase * 1.004)
            + 0.16 * table_sample(TRI_TABLE, phase * 0.5)
            + 0.08 * table_sample(SAW_TABLE, phase * 0.25 + mod_phase * 0.1)
        )
    return (
        0.74 * table_sample(SINE_TABLE, phase)
        + 0.14 * table_sample(SINE_TABLE, phase * 0.5)
        + 0.08 * table_sample(TRI_TABLE, phase * 1.5)
    )


def render_drum_event(
    left: array,
    right: array,
    start_frame: int,
    note: NoteEvent,
    length_frames: int,
    volume: float,
) -> None:
    length = max(1, min(length_frames, int(SAMPLE_RATE * 0.2)))
    if note.note == 36:
        freq = 84.0
        for i in range(length):
            t = i / SAMPLE_RATE
            env = max(0.0, 1.0 - i / length)
            current_freq = freq * (1.0 - 0.55 * (i / length))
            sample = math.sin(2.0 * math.pi * current_freq * t) * env * env * 0.9 * volume
            idx = start_frame + i
            if idx >= len(left):
                break
            left[idx] += sample
            right[idx] += sample
    elif note.note == 38:
        noise_step = 137.0
        phase = 0.0
        tone_phase = 0.0
        tone_step = 196.0 * TABLE_SIZE / SAMPLE_RATE
        for i in range(length):
            env = max(0.0, 1.0 - i / length)
            phase += noise_step
            tone_phase += tone_step
            sample = (
                0.72 * table_sample(NOISE_TABLE, phase)
                + 0.28 * table_sample(SINE_TABLE, tone_phase)
            ) * env * volume * 0.6
            idx = start_frame + i
            if idx >= len(left):
                break
            left[idx] += sample
            right[idx] += sample
    else:
        noise_step = 181.0
        phase = 0.0
        for i in range(length):
            env = max(0.0, 1.0 - i / length) ** 2
            phase += noise_step
            sample = table_sample(NOISE_TABLE, phase) * env * volume * 0.24
            idx = start_frame + i
            if idx >= len(left):
                break
            left[idx] += sample
            right[idx] += sample


def authored_voice_settings(kind: str) -> tuple[float, float, float, float]:
    if kind == "lead":
        return 0.02, 0.45, 0.0018, 0.30
    if kind == "comp":
        return 0.006, 0.55, 0.0012, 0.22
    if kind == "pad":
        return 0.22, 1.10, 0.0042, 0.26
    if kind == "counter":
        return 0.04, 0.65, 0.0014, 0.28
    return 0.01, 0.28, 0.0003, 0.20


def authored_voice_sample(
    kind: str,
    phase: float,
    mod_phase: float,
    lfo_phase: float,
    elapsed: float,
) -> float:
    if kind == "lead":
        tine = math.exp(-elapsed * 3.6)
        fm = table_sample(SINE_TABLE, mod_phase) * (11.0 * tine + 2.0)
        return (
            0.54 * table_sample(SINE_TABLE, phase + fm)
            + 0.18 * table_sample(SINE_TABLE, phase * 2.0 + fm * 0.34)
            + 0.12 * table_sample(TRI_TABLE, phase * 1.01)
            + 0.08 * table_sample(SINE_TABLE, phase * 0.5 + table_sample(SINE_TABLE, lfo_phase) * 4.0)
        )
    if kind == "comp":
        tine = math.exp(-elapsed * 5.0)
        fm = table_sample(SINE_TABLE, mod_phase) * (8.0 * tine + 1.6)
        return (
            0.48 * table_sample(SINE_TABLE, phase + fm)
            + 0.18 * table_sample(SINE_TABLE, phase * 3.0 + fm * 0.28)
            + 0.16 * table_sample(TRI_TABLE, phase)
            + 0.06 * table_sample(SINE_TABLE, phase * 0.5)
        )
    if kind == "pad":
        drift = table_sample(SINE_TABLE, lfo_phase * 0.6) * 12.0
        return (
            0.25 * table_sample(SINE_TABLE, phase * 0.996 + drift)
            + 0.25 * table_sample(SINE_TABLE, phase * 1.004 - drift)
            + 0.18 * table_sample(TRI_TABLE, phase * 0.5)
            + 0.10 * table_sample(SINE_TABLE, phase * 2.0)
            + 0.04 * table_sample(NOISE_TABLE, mod_phase * 0.3)
        )
    if kind == "counter":
        breath = table_sample(NOISE_TABLE, mod_phase * 0.45) * 0.08
        fm = table_sample(SINE_TABLE, mod_phase * 1.2) * 3.5
        return (
            0.58 * table_sample(SINE_TABLE, phase + fm)
            + 0.16 * table_sample(SINE_TABLE, phase * 2.0)
            + 0.10 * table_sample(TRI_TABLE, phase * 0.5)
            + breath
        )
    thunk = math.exp(-elapsed * 8.0)
    return (
        0.72 * table_sample(SINE_TABLE, phase)
        + 0.16 * table_sample(SINE_TABLE, phase * 0.5)
        + 0.08 * table_sample(TRI_TABLE, phase * 1.02)
        + 0.10 * table_sample(SINE_TABLE, phase * 2.0) * thunk
    )


def render_authored_drum_event(
    left: array,
    right: array,
    start_frame: int,
    note: NoteEvent,
    length_frames: int,
    volume: float,
) -> None:
    length = max(1, min(length_frames, int(SAMPLE_RATE * 0.28)))
    if note.note == 36:
        base_freq = 72.0
        for i in range(length):
            idx = start_frame + i
            if idx >= len(left):
                break
            t = i / SAMPLE_RATE
            env = max(0.0, 1.0 - i / length)
            bend = 1.0 - 0.72 * (i / length)
            tone = math.sin(2.0 * math.pi * base_freq * bend * t)
            click = table_sample(NOISE_TABLE, i * 37.0) * math.exp(-i / (0.015 * SAMPLE_RATE))
            sample = (tone * env * env * 0.95 + click * 0.08) * volume
            left[idx] += sample
            right[idx] += sample
    elif note.note == 38:
        tone_phase = 0.0
        tone_step = 188.0 * TABLE_SIZE / SAMPLE_RATE
        for i in range(length):
            idx = start_frame + i
            if idx >= len(left):
                break
            env = max(0.0, 1.0 - i / length)
            tone_phase += tone_step
            noise = table_sample(NOISE_TABLE, i * 149.0)
            tone = table_sample(SINE_TABLE, tone_phase)
            sample = (noise * 0.70 + tone * 0.22) * env * env * volume * 0.55
            left[idx] += sample * 0.96
            right[idx] += sample * 1.04
    else:
        for i in range(length):
            idx = start_frame + i
            if idx >= len(left):
                break
            env = max(0.0, 1.0 - i / length) ** 2
            noise = table_sample(NOISE_TABLE, i * 211.0)
            sample = noise * env * volume * 0.16
            left[idx] += sample * 0.92
            right[idx] += sample * 1.08


def apply_ping_pong_delay(
    left: array,
    right: array,
    delay_frames: int,
    feedback: float,
    cross_feedback: float,
    mix: float,
) -> None:
    for idx in range(delay_frames, len(left)):
        dry_left = left[idx]
        dry_right = right[idx]
        delayed_left = left[idx - delay_frames]
        delayed_right = right[idx - delay_frames]
        left[idx] = dry_left + (delayed_left * feedback + delayed_right * cross_feedback) * mix
        right[idx] = dry_right + (delayed_right * feedback + delayed_left * cross_feedback) * mix


def apply_diffuse_reverb(left: array, right: array, taps: list[tuple[float, float]]) -> None:
    for seconds, gain in taps:
        delay = max(1, int(seconds * SAMPLE_RATE))
        for idx in range(delay, len(left)):
            wet_left = (left[idx - delay] * 0.72 + right[idx - delay] * 0.28) * gain
            wet_right = (right[idx - delay] * 0.72 + left[idx - delay] * 0.28) * gain
            left[idx] += wet_left
            right[idx] += wet_right


def finalize_mix(left: array, right: array) -> float:
    low_left = 0.0
    low_right = 0.0
    for idx in range(len(left)):
        low_left += (left[idx] - low_left) * 0.018
        low_right += (right[idx] - low_right) * 0.018
        high_left = left[idx] - low_left
        high_right = right[idx] - low_right
        left[idx] = math.tanh((low_left * 0.96 + high_left * 0.68) * 0.96)
        right[idx] = math.tanh((low_right * 0.96 + high_right * 0.68) * 0.96)

    peak = max(
        max((abs(sample) for sample in left), default=1.0),
        max((abs(sample) for sample in right), default=1.0),
        1.0,
    )
    return 0.90 / peak


def write_wav(left: array, right: array, wav_path: Path, scale: float) -> None:
    wav_path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(wav_path), "wb") as handle:
        handle.setnchannels(2)
        handle.setsampwidth(2)
        handle.setframerate(SAMPLE_RATE)
        frames = bytearray()
        for left_sample, right_sample in zip(left, right):
            frames.extend(
                struct.pack(
                    "<hh",
                    int(left_sample * scale * 32767),
                    int(right_sample * scale * 32767),
                )
            )
        handle.writeframes(frames)


def render_authored_song_wav(spec: SongSpec, wav_path: Path) -> float:
    tracks, end_tick = arrange_song(spec)
    duration_seconds = tick_to_seconds(spec, end_tick) + 2.4
    total_frames = max(1, int(duration_seconds * SAMPLE_RATE))
    left = array("f", [0.0]) * total_frames
    right = array("f", [0.0]) * total_frames

    track_gain_by_kind = {
        "lead": 0.18,
        "comp": 0.15,
        "pad": 0.13,
        "bass": 0.16,
        "counter": 0.11,
        "drums": 0.13,
    }
    mod_ratio_by_kind = {
        "lead": 2.0,
        "comp": 3.0,
        "pad": 0.24,
        "bass": 1.0,
        "counter": 1.5,
    }

    for track in tracks:
        left_gain, right_gain = equal_power_pan(track.pan)
        track_gain = (track.volume / 127.0) * track_gain_by_kind.get(track.kind, 0.14)
        attack, release, detune, width = authored_voice_settings(track.kind)
        attack_frames = max(1, int(attack * SAMPLE_RATE))
        release_frames = max(1, int(release * SAMPLE_RATE))

        for event in track.notes:
            start_frame = int(tick_to_seconds(spec, event.start_tick) * SAMPLE_RATE)
            if start_frame >= total_frames:
                continue

            velocity_gain = (event.velocity / 127.0) ** 1.2
            if track.kind == "drums":
                drum_frames = max(1, int(tick_to_seconds(spec, event.duration_ticks) * SAMPLE_RATE))
                render_authored_drum_event(
                    left,
                    right,
                    start_frame,
                    event,
                    drum_frames,
                    track_gain * velocity_gain,
                )
                continue

            freq = midi_note_to_freq(event.note)
            base_duration = tick_to_seconds(spec, event.duration_ticks)
            frames = max(1, int((base_duration + release) * SAMPLE_RATE))
            phase_left = 0.0
            phase_right = 0.0
            mod_phase = 0.0
            lfo_phase = 0.0
            left_step = freq * (1.0 - detune) * TABLE_SIZE / SAMPLE_RATE
            right_step = freq * (1.0 + detune) * TABLE_SIZE / SAMPLE_RATE
            mod_step = freq * mod_ratio_by_kind.get(track.kind, 1.0) * TABLE_SIZE / SAMPLE_RATE
            lfo_step = 0.23 * TABLE_SIZE / SAMPLE_RATE

            for i in range(frames):
                idx = start_frame + i
                if idx >= total_frames:
                    break
                elapsed = i / SAMPLE_RATE
                env = envelope_value(i, frames, attack_frames, release_frames)
                motion = 1.0 + table_sample(SINE_TABLE, lfo_phase) * width
                phase_left += left_step * motion
                phase_right += right_step * motion
                mod_phase += mod_step
                lfo_phase += lfo_step
                sample_left = authored_voice_sample(track.kind, phase_left, mod_phase, lfo_phase, elapsed)
                sample_right = authored_voice_sample(
                    track.kind,
                    phase_right,
                    mod_phase + 11.0,
                    lfo_phase + 101.0,
                    elapsed,
                )
                left[idx] += sample_left * env * track_gain * velocity_gain * left_gain
                right[idx] += sample_right * env * track_gain * velocity_gain * right_gain

    beat_seconds = 60.0 / spec.bpm
    apply_ping_pong_delay(left, right, max(1, int(beat_seconds * 0.75 * SAMPLE_RATE)), 0.42, 0.20, 0.18)
    apply_ping_pong_delay(left, right, max(1, int(beat_seconds * 1.5 * SAMPLE_RATE)), 0.18, 0.32, 0.10)
    apply_diffuse_reverb(left, right, [(0.11, 0.08), (0.17, 0.06), (0.23, 0.05), (0.31, 0.04)])
    scale = finalize_mix(left, right)
    write_wav(left, right, wav_path, scale)
    return duration_seconds


def render_song_wav(spec: SongSpec, wav_path: Path) -> float:
    tracks, end_tick = arrange_song(spec)
    duration_seconds = tick_to_seconds(spec, end_tick) + 1.2
    total_frames = max(1, int(duration_seconds * SAMPLE_RATE))
    left = array("f", [0.0]) * total_frames
    right = array("f", [0.0]) * total_frames

    for track in tracks:
        left_gain, right_gain = equal_power_pan(track.pan)
        track_gain = (track.volume / 127.0) * 0.21
        if track.kind == "pad":
            attack, release = 0.18, 0.42
        elif track.kind == "lead":
            attack, release = 0.02, 0.12
        elif track.kind == "comp":
            attack, release = 0.01, 0.14
        else:
            attack, release = 0.01, 0.10

        attack_frames = max(1, int(attack * SAMPLE_RATE))
        release_frames = max(1, int(release * SAMPLE_RATE))

        for event in track.notes:
            start_frame = int(tick_to_seconds(spec, event.start_tick) * SAMPLE_RATE)
            if start_frame >= total_frames:
                continue
            if track.kind == "drums":
                drum_frames = max(1, int(tick_to_seconds(spec, event.duration_ticks) * SAMPLE_RATE))
                render_drum_event(left, right, start_frame, event, drum_frames, track_gain)
                continue

            freq = midi_note_to_freq(event.note)
            base_duration = tick_to_seconds(spec, event.duration_ticks)
            frames = max(1, int((base_duration + release) * SAMPLE_RATE))
            velocity_gain = (event.velocity / 127.0) ** 1.3
            phase = 0.0
            mod_phase = 0.0
            phase_step = freq * TABLE_SIZE / SAMPLE_RATE
            mod_step = 5.0 * TABLE_SIZE / SAMPLE_RATE
            for i in range(frames):
                idx = start_frame + i
                if idx >= total_frames:
                    break
                env = envelope_value(i, frames, attack_frames, release_frames)
                phase += phase_step
                mod_phase += mod_step
                sample = synth_voice(track.kind, freq, phase, mod_phase) * env * track_gain * velocity_gain
                left[idx] += sample * left_gain
                right[idx] += sample * right_gain

    delay_a = int(0.17 * SAMPLE_RATE)
    delay_b = int(0.31 * SAMPLE_RATE)
    for idx in range(delay_b, total_frames):
        left[idx] += left[idx - delay_a] * 0.12 + left[idx - delay_b] * 0.06
        right[idx] += right[idx - delay_a] * 0.12 + right[idx - delay_b] * 0.06

    shimmer = int(0.43 * SAMPLE_RATE)
    for idx in range(shimmer, total_frames):
        left[idx] += right[idx - shimmer] * 0.04
        right[idx] += left[idx - shimmer] * 0.04

    smooth_left = 0.0
    smooth_right = 0.0
    for idx in range(total_frames):
        smooth_left += (left[idx] - smooth_left) * 0.18
        smooth_right += (right[idx] - smooth_right) * 0.18
        left[idx] = math.tanh(left[idx] * 0.78 + smooth_left * 0.22)
        right[idx] = math.tanh(right[idx] * 0.78 + smooth_right * 0.22)

    peak = max(
        max((abs(sample) for sample in left), default=1.0),
        max((abs(sample) for sample in right), default=1.0),
        1.0,
    )
    scale = 0.92 / peak

    wav_path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(wav_path), "wb") as handle:
        handle.setnchannels(2)
        handle.setsampwidth(2)
        handle.setframerate(SAMPLE_RATE)
        frames = bytearray()
        for left_sample, right_sample in zip(left, right):
            frames.extend(struct.pack("<hh", int(left_sample * scale * 32767), int(right_sample * scale * 32767)))
        handle.writeframes(frames)
    return duration_seconds


def encode_mp3(wav_path: Path, mp3_path: Path) -> None:
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        raise RuntimeError("ffmpeg not found in PATH")
    subprocess.run(
        [
            ffmpeg,
            "-y",
            "-loglevel",
            "error",
            "-i",
            str(wav_path),
            "-codec:a",
            "libmp3lame",
            "-q:a",
            "3",
            str(mp3_path),
        ],
        check=True,
    )


def wav_duration_seconds(wav_path: Path) -> float:
    with wave.open(str(wav_path), "rb") as handle:
        frames = handle.getnframes()
        sample_rate = handle.getframerate()
    return frames / sample_rate if sample_rate else 0.0


def candidate_fluidsynth_paths() -> list[Path]:
    candidates: list[Path] = []
    if exe := shutil.which("fluidsynth"):
        candidates.append(Path(exe))
    project_root = Path(__file__).resolve().parents[1]
    candidates.extend(
        [
            project_root / "tools" / "fluidsynth-portable" / "fluidsynth-v2.5.2-win10-x64-cpp11" / "bin" / "fluidsynth.exe",
            project_root / "tools" / "fluidsynth-portable" / "fluidsynth-v2.5.2-win10-x64-glib" / "bin" / "fluidsynth.exe",
        ]
    )
    return [path for path in candidates if path.exists()]


def candidate_soundfonts() -> list[Path]:
    candidates: list[Path] = []
    if env_path := os.environ.get("TC2000_SOUNDFONT"):
        candidates.append(Path(env_path))
    candidates.extend(
        [
            Path("C:/Program Files/MuseScore 4/sound/MS Basic.sf3"),
            Path("C:/Program Files/MuseScore 4/sound/MuseScore_General.sf2"),
        ]
    )
    return [path for path in candidates if path.exists()]


def render_song_with_fluidsynth(midi_path: Path, wav_path: Path) -> bool:
    fluidsynth = next(iter(candidate_fluidsynth_paths()), None)
    soundfont = next(iter(candidate_soundfonts()), None)
    if fluidsynth is None or soundfont is None:
        return False

    wav_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            str(fluidsynth),
            "-ni",
            "-q",
            "-g",
            "0.35",
            "-F",
            str(wav_path),
            "-T",
            "wav",
            "-r",
            "44100",
            str(soundfont),
            str(midi_path),
        ],
        check=True,
    )
    return wav_path.exists() and wav_path.stat().st_size > 4096


def write_pack(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    for spec in active_songs():
        midi = make_song(spec)
        midi_path = out_dir / f"{spec.slug}.mid"
        midi_path.write_bytes(midi)
        wav_path = out_dir / f"{spec.slug}.wav"
        mp3_path = out_dir / f"{spec.slug}.mp3"
        if not render_song_with_fluidsynth(midi_path, wav_path):
            duration_seconds = render_song_wav(spec, wav_path)
        else:
            duration_seconds = wav_duration_seconds(wav_path)
        encode_mp3(wav_path, mp3_path)
        manifest.append(
            {
                "midi_file": midi_path.name,
                "audio_file": mp3_path.name,
                "runtime_audio_file": wav_path.name if spec.role == "start_theme" else None,
                "title": spec.title,
                "role": spec.role,
                "bpm": spec.bpm,
                "bars": spec.bars,
                "duration_seconds": round(duration_seconds, 2),
                "description": spec.description,
                "copyright_note": "Original composition generated specifically for this project.",
            }
        )
        if spec.role != "start_theme":
            wav_path.unlink(missing_ok=True)
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")


if __name__ == "__main__":
    project_root = Path(__file__).resolve().parents[1]
    write_pack(project_root / "assets" / "music")
