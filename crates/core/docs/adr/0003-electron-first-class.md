# Electron is a first-class domain concept

Electron applications are detected heuristically (walk `--type=` argv chains to a root validated by exe path or `.asar` presence), tagged `is_electron`, given a friendly `electron_app_name`, and promoted to a first-class citizen: a `Filter::Electron`, a toolbar tag, and participation in search scoring across `name`/`command`/`electron_app_name`.

Most system monitors treat Electron processes as noise to merge or hide. Here they are surfaced and searchable — a deliberate choice driven by the author's usage, costing a per-tick detection pass and a `--type=` heuristic that can misclassify non-Electron multi-process apps.
