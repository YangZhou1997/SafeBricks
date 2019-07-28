#ifndef __HUGEPAGE_CONFIG_H
#define __HUGEPAGE_CONFIG_H
#include <sys/stat.h>

#define PFN_MASK_SIZE	8
#define RTE_BAD_PHYS_ADDR ((phys_addr_t)-1)
#define PATH_MAX        4096	/* # chars in a path name including nul */
#define MAX_HUGEPAGE_PATH PATH_MAX
/**
 * add a byte-value offset from a pointer
 */
#define RTE_PTR_ADD(ptr, x) ((void*)((uintptr_t)(ptr) + (x)))


const char *default_hugepage_config_path = "/var/run/.netbricks-outside_hugepage_info";

const char *eal_hugepage_info_path(void)
{
    return default_hugepage_config_path;
}

/*
 * uses fstat to report the size of a file on disk
 */
off_t getFileSize(int fd)
{
	struct stat st;
	if (fstat(fd, &st) < 0)
		return 0;
	return st.st_size;
}


/**
 * Structure used to store informations about hugepages that we mapped
 * through the files in hugetlbfs.
 */
struct hugepage_file {
	void *orig_va;      /**< virtual addr of first mmap() */
	void *final_va;     /**< virtual addr of 2nd mmap() */
	uint64_t physaddr;  /**< physical addr */
	size_t size;        /**< the page size */
	int socket_id;      /**< NUMA socket ID */
	int file_id;        /**< the '%d' in HUGEFILE_FMT */
	int memseg_id;      /**< the memory segment to which page belongs */
	char filepath[MAX_HUGEPAGE_PATH]; /**< path to backing file on filesystem */
};


#endif // __HUGEPAGE_CONFIG_H