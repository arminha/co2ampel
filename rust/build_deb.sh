#!/bin/bash
set -e

SCRIPT_DIR=$(dirname $0)

rm -rf build
mkdir build

rsync -av --exclude-from=$SCRIPT_DIR/.gitignore \
      --exclude=.git --exclude=build --exclude=apt --exclude=cargo \
      $SCRIPT_DIR/ build/co2ampel-backend

pushd build
pushd co2ampel-backend

# create changelog
AUTHOR="Armin Häberling <armin.aha@gmail.com>"
VERSION=`cargo metadata --format-version 1 --no-deps | python3 -c "import sys, json; print(json.load(sys.stdin)['packages'][0]['version'])"`

cat > debian/changelog << EOF
co2ampel-backend ($VERSION) unstable; urgency=low

  * Packaged ${VERSION}

 -- ${AUTHOR}  $(date -R)

EOF

dpkg-buildpackage -b -rfakeroot -us -uc
popd
lintian --suppress-tags possible-gpl-code-linked-with-openssl co2ampel-backend_*.changes
popd
