#!/bin/bash

HG_GIT_FAST_IMPORT_VOLUME=${HG_GIT_FAST_IMPORT_VOLUME:-"`pwd`:/repositories"}

docker run -v $HG_GIT_FAST_IMPORT_VOLUME -it --rm kilork/hg-git-fast-import hg-git-fast-import "$@"