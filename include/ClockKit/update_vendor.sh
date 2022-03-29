for f in ./*.cpp ./*.hpp ./*.h; do
    if [[ $(diff -q "$f" "${HOME}/src/public/github/clockkit/ClockKit/${f##*/}"; echo $?) ]]; then
        diff -q "$f" "${HOME}/src/public/github/clockkit/ClockKit/${f##*/}"
        cp "${HOME}/src/public/github/clockkit/ClockKit/${f##*/}" .
    else
        echo "SAME"
        diff -q "$f" "${HOME}/src/public/github/clockkit/ClockKit/${f##*/}"
    fi
done
