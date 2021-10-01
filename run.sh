#!/bin/bash
#-v /opt/sysroots:/opt/sysroots \

echo "Running docker "
docker run -ti --rm \
    --net="host" \
    --privileged \
    -v $(pwd)/../../:/home/envision/production \
    -v /nfsboot/rpu_mockup_app/:/rpu_mockup_app \
    -v /dev/:/dev \
    envision/dev-cross-aarch64:latest /bin/bash
