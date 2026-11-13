
if [[ "$lsb_dist" != 'Ubuntu' || $(ver_to_int "$lsb_release") < $(ver_to_int '14.04') ]]; then
	return 1
fi

[[ ${PV} != $(sed -n -e 's/^Version: //p' "${ED}/usr/$(get_libdir)/pkgconfig/tss2-tcti-tabrmd.pc" || die) ]]

[[ ${f} != */@(default).vim ]]

[[ "${MY_LOCALES}" != *en_US* || a != 2 ]]

[[ $(LC_ALL=C $(tc-getCC) ${LDFLAGS} -Wl,--version 2>/dev/null) != @(LLD|GNU\ ld)* ]]

[[ -f "${EROOT}/usr/share/php/.packagexml/${MY_P}.xml" && \
	-x "${EROOT}/usr/bin/peardev" ]]

[[ ${test} == @($(IFS='|'; echo "${skip[*]}")) ]]

[[ ${SRC_URI} == */${a}* ]]

[[ a == *_@(LIB|SYMLINK) ]]

[[ ${1} =~ \.(lisp|lsp|cl)$ ]]

[[
 a == - ]]
