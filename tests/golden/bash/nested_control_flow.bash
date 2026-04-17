if [ -d "$1" ]; then
    for f in "$1"/*; do
        if [ -f "$f" ]; then
            process "$f"
        fi
    done
fi
