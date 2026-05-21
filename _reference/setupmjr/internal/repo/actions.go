package repo

var actionWorkflows = map[string]string{
	"prebuild-devcontainer": `name: 'Pre-build Dev Container'

on:
  push:
    tags:
      - 'v*'
      - '*-v*'
  workflow_dispatch:
    inputs:
      tag:
        description: 'Tag to build (e.g. v1.2.3 or main-v1.2.3)'
        required: true

jobs:
  build:
    runs-on: ubuntu-latest
    concurrency:
      group: ${{ github.workflow }}-${{ github.ref }}
      cancel-in-progress: true
    permissions:
      contents: write
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: MiguelRodo/actions/prebuild-devcontainer@v2
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          tag: ${{ github.event.inputs.tag }}
`,
	"add-issues-to-project": `name: Sync Issues to Project

on:
  workflow_dispatch:
  issues:
    types: [opened, reopened]

jobs:
  add-to-project:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: MiguelRodo/actions/add-issues-to-project@v2
        with:
          ADD_ISSUES_TO_PROJECT_TOKEN: ${{ secrets.ADD_ISSUES_TO_PROJECT_TOKEN }}
          # project_name: "My Custom Project Board"
          # is_project_owner_org: "true"
`,
	"version-release": `name: Version and Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'
  workflow_dispatch:
    inputs:
      version:
        description: 'Exact version (e.g. 1.2.3). Cannot be used with bump_type.'
        required: false
      bump_type:
        description: 'Component to bump: major | minor | patch. Cannot be used with version.'
        required: false
      python_version:
        description: 'Override: exact version for the Python package.'
        required: false
      r_version:
        description: 'Override: exact version for the R package.'
        required: false

jobs:
  version-release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: MiguelRodo/actions/version-release@v2
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          version: ${{ inputs.version }}
          bump_type: ${{ inputs.bump_type }}
          python_version: ${{ inputs.python_version }}
          r_version: ${{ inputs.r_version }}
`,
	"go-version-release": `name: Go Version and Release

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Exact version (e.g. 1.2.3). Cannot be used with bump_type.'
        required: false
      bump_type:
        description: 'Component to bump: major | minor | patch. Cannot be used with version.'
        required: false
      go_version:
        description: 'Go version to install (defaults to 1.22).'
        required: false
      goreleaser_config:
        description: 'Optional path to the GoReleaser config file.'
        required: false
      apt_repo:
        description: 'Optional target GitHub repository in owner/name form for publishing generated .deb artifacts.'
        required: false
      apt_repo_token:
        description: 'Optional token for apt_repo access when publishing to a different repository.'
        required: false

jobs:
  release:
    runs-on: ubuntu-latest # Linux runner required
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: MiguelRodo/actions/go-version-release@v2
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          apt_repo_token: ${{ secrets.APT_REPO_TOKEN }}
          version: ${{ inputs.version }}
          bump_type: ${{ inputs.bump_type }}
          go_version: ${{ inputs.go_version }}
          goreleaser_config: ${{ inputs.goreleaser_config }}
          apt_repo: ${{ inputs.apt_repo }}
`,
	"r-version-release": `name: R Version and Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'
  workflow_dispatch:
    inputs:
      version:
        description: 'Exact version (e.g. v1.2.3). Cannot be used with bump_type.'
        required: false
      bump_type:
        description: 'Component to bump: major | minor | patch. Cannot be used with version.'
        required: false
      version_force:
        description: 'When true, skip strict version progression checks.'
        required: false
        type: boolean

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: MiguelRodo/actions/r-version-release@v2
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          version: ${{ inputs.version }}
          bump_type: ${{ inputs.bump_type }}
          version_force: ${{ inputs.version_force }}
`,
	"apt-repo-prune": `name: Prune APT Repository

on:
  workflow_dispatch:
    inputs:
      retention:
        description: 'Retention policy: latest | latest-per-minor | latest-per-major'
        required: false
        default: latest-per-major
  schedule:
    - cron: '0 3 * * 0'   # weekly on Sunday at 03:00 UTC

jobs:
  prune:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: MiguelRodo/actions/apt-repo-prune@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          retention: ${{ inputs.retention || 'latest-per-major' }}
          apt_signing_key: ${{ secrets.APT_SIGNING_KEY }}
          apt_signing_key_passphrase: ${{ secrets.APT_SIGNING_KEY_PASSPHRASE }}
`,
	"publish-quarto-site": `name: Publish Quarto Site

on:
  push:
    branches: [main]

permissions:
  contents: write
  pages: write

jobs:
  build-and-publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: MiguelRodo/actions/publish-quarto-site@v2
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
`,
	"sync-issues-to-project": `name: Sync Issues to Project

on:
  workflow_dispatch:
  issues:
    types: [opened, reopened]

jobs:
  add-to-project:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: MiguelRodo/actions/add-issues-to-project@v2
        with:
          ADD_ISSUES_TO_PROJECT_TOKEN: ${{ secrets.ADD_ISSUES_TO_PROJECT_TOKEN }}
          # project_name: "My Custom Project Board"
          # is_project_owner_org: "true"
`,
}
