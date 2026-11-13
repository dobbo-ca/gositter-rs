UBE_ROOT}/cmd"; find . -mindepth 1 -maxdepth 1 -type d | cut -c 3-)
CMD_FLAG=false

echo "--------------------------------------------------------------------------------"
for tar in ${ALL_TARGETS}; do
	for cmdtar in ${CMD_TARGETS}; do
		if [ "${tar}" = "${cmdtar}" ]; then
			if [ ${CMD_FLAG} = true ]; then
				continue 2;
			fi

			echo -e "${red}${CMD_TARGETS}${reset}"
			make -C "${KUBE_ROOT}" "${tar}" PRINT_HELP=y
			echo "---------------------------------------------------------------------------------"

			CMD_FLAG=true
			continue 2
		fi
	done

	echo -e "${red}${tar}${reset}"
	make -C "${KUBE_ROOT}" "${tar}" PRINT_HELP=y
	echo "---------------------------------------------------------------------------------"
done
