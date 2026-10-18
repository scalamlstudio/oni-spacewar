#!/bin/sh
set -eu
cd "$(dirname "$0")"
swift prepare-batch-b-v1.swift bunk-bed-source-v1.png bunk-bed-v1.png 128 384 232
swift prepare-batch-b-v1.swift locker-source-v1.png locker-v1.png 128 320 244
swift prepare-batch-b-v1.swift table-source-v1.png table-v1.png 256 192 116
swift prepare-batch-b-v1.swift stool-source-v1.png stool-v1.png 128 192 100
swift prepare-batch-b-v1.swift sofa-source-v1.png sofa-v1.png 256 224 136
swift prepare-batch-b-v1.swift cat-bed-source-v1.png cat-bed-v1.png 128 160 84
swift prepare-batch-b-v1.swift cat-tree-source-v1.png cat-tree-v1.png 128 352 264
swift prepare-batch-b-v1.swift scratch-post-source-v1.png scratch-post-v1.png 128 224 144
swift prepare-batch-b-v1.swift food-bowls-source-v1.png food-bowls-v1.png 128 160 76
swift prepare-batch-b-v1.swift crate-stack-source-v1.png crate-stack-v1.png 128 224 154
swift prepare-batch-b-v1.swift snack-dispenser-source-v1.png snack-dispenser-v1.png 128 288 204
swift prepare-batch-b-v1.swift aquarium-source-v1.png aquarium-v1.png 256 256 164
swift prepare-batch-b-v1.swift potted-plant-source-v1.png potted-plant-v1.png 128 224 144
swift prepare-batch-b-v1.swift hydro-planter-source-v1.png hydro-planter-v1.png 256 192 109
swift prepare-batch-b-v1.swift floor-lamp-source-v1.png floor-lamp-v1.png 128 288 224
swift prepare-batch-b-v1.swift rug-source-v1.png rug-v1.png 256 256 248
swift prepare-batch-b-v1.swift wall-screen-source-v1.png wall-screen-v1.png 256 96 88
swift prepare-batch-b-v1.swift wall-light-source-v1.png wall-light-v1.png 128 96 88
swift prepare-batch-b-v1.swift tool-rack-source-v1.png tool-rack-v1.png 128 96 88
swift inspect-batch-b-v1.swift .
