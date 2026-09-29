# Design Loops, Local Models and Evidence Capture

- **Date.** 2026-09-28
- **Method.** Three parallel web-research agents, each answering one question from primary sources (arXiv, vendor engineering posts, tool documentation), reported with URLs and dates. Claims below are the sources' claims about their own systems; nothing here was reproduced. One PDF (VizAssert) was read from its abstract page only.
- **Hardware in question.** WSL2 on an i7-10750H, 15 GB RAM, RTX 3060 Laptop 6 GB with `/dev/dxg` passthrough.

## 1. Loop Shapes With Evidence

- **Separate critic on rendered evidence, few rounds.** Generator, VLM critic on the screenshot, consolidator: up to +17.8% on WebDev Arena prompts over three rounds; LoRA-distilling the loop kept ~25% of the gain ([2604.05839](https://arxiv.org/abs/2604.05839), Apr 2026). Self-revision by the same model gave minor gains for strong models and none for weak ones ([Design2Code, 2403.03163](https://arxiv.org/abs/2403.03163)).
- **Reject before judging.** Zero reward for output that fails to render ([ReLook, 2510.11498](https://arxiv.org/abs/2510.11498)); compiler + CLIP + dedup filters took UICoder from 0.4% to 79% compilable, then DPO over survivors ([2406.07739](https://arxiv.org/html/2406.07739v1)); v0's mid-stream and AST autofixers under 250 ms gave a double-digit success gain ([Vercel](https://vercel.com/blog/how-we-made-v0-an-effective-coding-agent), Jan 2026).
- **Relative, not absolute.** UI2Code^N optimizes relative rankings because absolute visual scores are noisy ([2511.08195](https://arxiv.org/abs/2511.08195)); pairwise agrees with humans more than pointwise ([2502.06193](https://arxiv.org/pdf/2502.06193)).
- **One repair per round with history.** Local edits propagate through layout and undo earlier fixes; rubric-scoped repair is more stable across six VLMs ([RubSE, 2608.24138](https://arxiv.org/abs/2608.24138)).
- **Score interaction, backtrack to best.** Screenshot score plus GUI-agent functional score as step rewards: 26.4% to 51.9% on WebGen-Bench ([2509.22644](https://arxiv.org/abs/2509.22644)). Top visual model ranked 4th on interaction ([UI2App, 2607.06306](https://arxiv.org/abs/2607.06306)).
- **Checklist judging in a sandbox.** Three interaction screenshots, ten-dimension checklist, two referees: 94.4% ranking agreement with WebDev Arena, κ>0.85 vs experts ([ArtifactsBench, 2507.04952](https://arxiv.org/abs/2507.04952)).
- **Evolutionary archives.** AlphaEvolve: MAP-Elites archive, evaluator cascades, prompts sampled from the archive ([2506.13131](https://arxiv.org/abs/2506.13131)). No UI-specific instance published; in-context QD shows LLMs can fill archives ([2404.15794](https://arxiv.org/abs/2404.15794)).

## 2. Failure Modes

- Twelve catalogued judge biases ([2410.02736](https://arxiv.org/abs/2410.02736)); order swap alone moves pairwise code verdicts >10% ([IJCNLP 2025](https://aclanthology.org/2025.ijcnlp-long.18/)); self-preference is uncorrelated with capability and cut 31.5% by decomposed rubrics ([2604.22891](https://arxiv.org/abs/2604.22891)).
- A trace saying "tool succeeded" made open VLM judges accept 78–90% of visibly failed outputs vs 7–19% without ([2609.28564](https://arxiv.org/abs/2609.28564), Sep 2026).
- Agents reached 100% pass hiding 68% true capability; mitigations are hermetic sandboxes, acceptance checks that outrank judges, frozen holdouts and canaries ([2609.02246](https://arxiv.org/abs/2609.02246)).
- Judge audits over 541k judgments: exact-match agreement overstates κ by 33–41 points; test–retest >0.95 coexists with position bias >0.10; rankings shift up to 14 places ([2606.19544](https://arxiv.org/abs/2606.19544)). JudgeBench's verified pairs: GPT-4o barely beat chance ([2410.12784](https://arxiv.org/abs/2410.12784)).
- RL on visual repair trajectories: "limited and unstable gains" ([1D-Bench, 2602.18548](https://arxiv.org/abs/2602.18548)).
- Frontier VLM judges reach ~0.49 Pearson with humans on UI; open models 0.03–0.23; LLaVA copied the presented order 88% of the time ([MLLM-as-a-Judge, 2402.04788](https://arxiv.org/html/2402.04788)). Designers agree with each other at Krippendorff α=0.25; personalized judges beat aggregated ones with 20× fewer examples ([DesignPref, 2511.20513](https://arxiv.org/abs/2511.20513)).

## 3. Capturing Judgments as Evidence

- **Bradley–Terry with intervals.** Chatbot Arena: reweighted MLE, sandwich 95% CIs, active pair sampling cut votes from 6,800 to 4,400 for the same precision; crowd–expert agreement 72–83% vs expert–expert 79–90% ([2403.04132](https://arxiv.org/abs/2403.04132)); code at [lmarena/arena-rank](https://github.com/lmarena/arena-rank). Design Arena: ≥15 comparisons before ranking, Wilson CI ([methodology](https://notes.designarena.ai/methodology/)).
- **Small designer datasets move models.** 1,460 annotations from 21 designers, UIClip reward model, ORPO: 59% win rate over base in a 405-comparison expert arena; sketch feedback gave 63.6% agreement vs 49.2% for rankings ([2509.16779](https://arxiv.org/html/2509.16779), CHI 2026).
- **Deterministic gates.** Lighthouse CI budgets vs assertions with warn/error; axe-core automates ~57% of WCAG issues ([ACT coverage](https://jangwook.net/en/blog/en/act-rules-axe-coverage-wcag-sc-2026/)); Playwright `toHaveScreenshot` (pixelmatch, masks); token lint with SARIF ([design-lint](https://lapidist.net/articles/2025/introducing-lapidist-design-lint/)); layout as SMT: VizAssert checked 14 assertions over 62 pages, 64 true errors, 13 false positives ([PLDI 2018](https://homes.cs.washington.edu/~mernst/pubs/verify-layout-pldi2018-abstract.html)); pixel display-bug detectors at ~0.84 precision/recall ([OwlEye](https://arxiv.org/abs/2009.01417), [Nighthawk](https://arxiv.org/abs/2205.13945)).
- **Unproven.** LLM critique still short of experts on UICrit ([2407.08850](https://arxiv.org/abs/2407.08850)); MAP-Elites for UI has no descriptor study; aesthetic predictors (LAION, PickScore, ImageReward, Q-Align) are trained on photos and unvalidated on UI screenshots.

## 4. Small Local Models on the Stated Hardware

| Model | Quant on disk | 6 GB fit | Use |
| --- | --- | --- | --- |
| Qwen3-VL-4B-Instruct | Q4_K_M ~2.5 GB + mmproj | resident | structured screenshot questions, OCR of readouts |
| Qwen3-VL-8B / Qwen3.5-9B | ~5–6.5 GB | CPU offload, ~10–20 tok/s (estimate) | better critique prose, slower |
| Gemma 4 E4B | Q4_K_M ~5 GB | tight | vision + text |
| Ferret-UI Lite 3B | fp16 ~6 GB | offload | GUI grounding (ScreenSpot-V2 91.6%) |
| Qwen3-4B / 8B text | ~2.5 / 5 GB | resident / offload | mechanical CSS and HTML edits |
| SigLIP / CLIP | small | resident | near-duplicate detection; a Bradley–Terry head on frozen embeddings trains on CPU |

Stack: llama.cpp CUDA build; Ollama cannot load the Qwen3.5 vision GGUFs and is 3–10% slower; vLLM and SGLang need more than 6 GB; never install a Linux NVIDIA driver inside WSL. Screenshot prefill (1–2k image tokens per 1440×900 shot) dominates latency. QLoRA of a 3–4B VLM does not fit in 6 GB with screenshot-length sequences.

**Verdict.** Local models are good enough for grounding, element detection, OCR, fixed yes/no structured checks, first-pass filtering and mechanical edits. They are not good enough for aesthetic judgment or ranking (≤8B open models trail frontier by ~10 points and show heavy position and length bias). Sources: [Qwen3-VL-4B GGUF](https://huggingface.co/Qwen/Qwen3-VL-4B-Instruct-GGUF), [Qwen3.5](https://unsloth.ai/docs/models/qwen3.5), [Gemma 4 E4B](https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF), [Ferret-UI Lite](https://arxiv.org/abs/2509.26539), [AesEval-Bench](https://arxiv.org/html/2603.01083), [UI-Bench](https://arxiv.org/abs/2508.20410).

## 5. Mechanisms Adopted

1. Render gate with zero credit before any judge (contract clause 2).
2. Pairwise, both orders, ties one half, renders only, decomposed rubric (clause 3).
3. Bradley–Terry with intervals and a comparison floor (clause 4).
4. Judge admission by known-answer pairs, position-flip rate and agreement with owner Votes (clause 5).
5. Checks outrank Judgments; every Check ships with a negative control (clause 1, 7).
6. Owner Votes collected blind from the start; a personalized preference head is the planned second opinion.
7. Local models as a filter tier with a measured false-positive rate, never a ranking judge.
8. A quality-diversity archive is an experiment with a pre-registered protocol, not a commitment.
