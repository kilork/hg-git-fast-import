profile:
	cargo build --release --example private_single_stable
	cp callgrind.annotate callgrind.annotate.`date '+%Y%m%d%H%M%S'` || true
	valgrind --callgrind-out-file=callgrind.profile --tool=callgrind  target/release/examples/private_single_stable
	callgrind_annotate --auto=yes --inclusive=yes --tree=both callgrind.profile > callgrind.annotate
	less callgrind.annotate