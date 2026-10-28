#!/usr/bin/env python3
"""Independent Python reference model plus exhaustive small-input invariants.

Run: python3 tools/validate_reference.py
"""
import itertools
import unittest

TOKENS = ("RAM", "KON", "nRAM", "nKON")
FLIP = dict(zip(TOKENS, ("KON", "RAM", "nKON", "nRAM")))
COMMANDS = {"RAM", "nRAM"}


def step(tokens):
    results = []
    for i, op in enumerate(tokens):
        if op == "RAM":
            results.extend(tokens[i + 1:])
        elif op == "nRAM":
            results.extend(FLIP[token] for token in tokens[i + 1:])
    return tuple(results)


def simulate(tokens, max_rounds=30, max_tokens=10000):
    tokens = tuple(tokens)
    seen = {tokens: 0}
    rounds = 0
    while True:
        if not any(t in COMMANDS for t in tokens):
            return "done", rounds, tokens
        next_size = sum(len(tokens) - i - 1 for i, t in enumerate(tokens) if t in COMMANDS)
        if next_size > max_tokens:
            return "token_limit", rounds, tokens
        tokens = step(tokens)
        rounds += 1
        if not any(t in COMMANDS for t in tokens):
            return "done", rounds, tokens
        if tokens in seen:
            return "cycle", rounds, tokens
        if rounds >= max_rounds:
            return "round_limit", rounds, tokens
        seen[tokens] = rounds


class ReferenceTests(unittest.TestCase):
    def test_whiteboard(self):
        initial = ("RAM", "nRAM", "KON")
        one = step(initial)
        two = step(one)
        self.assertEqual(one, ("nRAM", "KON", "RAM"))
        self.assertEqual(two, ("RAM", "KON"))
        self.assertEqual(simulate(initial), ("done", 3, ("KON",)))

    def test_other_examples(self):
        self.assertEqual(simulate(("RAM", "RAM", "KON")), ("done", 2, ("KON", "KON")))
        self.assertEqual(simulate(("RAM", "RAM", "RAM"))[0], "cycle")
        self.assertEqual(simulate(("nRAM", "nRAM")), ("done", 1, ("nKON",)))
        self.assertEqual(simulate(("RAM",)), ("done", 1, ()))

    def test_all_inputs_up_to_six_tokens(self):
        for length in range(1, 7):
            for tokens in itertools.product(TOKENS, repeat=length):
                outcome, rounds, final = simulate(tokens, max_rounds=30, max_tokens=1000)
                self.assertIn(outcome, {"done", "cycle", "token_limit", "round_limit"})
                self.assertLessEqual(len(final), 1000)
                self.assertLessEqual(rounds, 30)
                if outcome == "done":
                    self.assertTrue(all(token not in COMMANDS for token in final))

    def test_formula_matches_explicit_step(self):
        for length in range(1, 7):
            for tokens in itertools.product(TOKENS, repeat=length):
                estimated = sum(length - i - 1 for i, t in enumerate(tokens) if t in COMMANDS)
                self.assertEqual(estimated, len(step(tokens)))


if __name__ == "__main__":
    unittest.main(verbosity=2)
