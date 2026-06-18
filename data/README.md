# data/

This directory holds sample and test match-report CSV files used during local
development.

**Its contents are git-ignored** (see the repository [`.gitignore`](../.gitignore))
so that potentially proprietary or customer data is never committed or
distributed with the project. Only this `README.md` is tracked, which keeps the
directory present in a fresh clone.

## Adding your own samples

Drop Physna match-report CSV exports here, for example:

```
data/test-report.csv
```

The tool's tests and examples look for files in this directory. Because the data
is ignored, contributors must supply their own samples; do **not** add real data
to the repository.
