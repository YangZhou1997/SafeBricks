#include <stdio.h>
#include <stdint.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <errno.h>
#include <stdbool.h>
#include <unistd.h>
 #include <string.h>
#include "mem_config.h"
#include "hugepage_config.h"

int mem_cfg_fd = -1;

/* early configuration structure, when memory config is not mmapped */
struct rte_mem_config early_mem_config;

/* Address of global and public configuration */
struct rte_config rte_config = {
		.mem_config = &early_mem_config,
};

/* attach to an existing shared memory config */
void rte_eal_config_attach(void)
{
	struct rte_mem_config *mem_config;

	const char *pathname = eal_runtime_config_path();

	if (mem_cfg_fd < 0){
		mem_cfg_fd = open(pathname, O_RDWR);
		if (mem_cfg_fd < 0)
			printf("Cannot open '%s' for rte_mem_config\n", pathname);
	}
    // printf("after open\n");
	/* map it as read-only first */
	mem_config = (struct rte_mem_config *) mmap(NULL, sizeof(*mem_config),
			PROT_READ, MAP_SHARED, mem_cfg_fd, 0);
    // printf("after mmap\n");
	
    if (mem_config == MAP_FAILED)
		printf("Cannot mmap memory for rte_config! error %i (%s)\n",
			  errno, strerror(errno));

	rte_config.mem_config = mem_config;
}

void rte_eal_mcfg_wait_complete(struct rte_mem_config* mcfg)
{
	/* wait until shared mem_config finish initialising */
	while(mcfg->magic != RTE_MAGIC)
		rte_pause();
}

/* reattach the shared config at exact memory location primary process has it */
void rte_eal_config_reattach(void)
{
    // printf("reattach\n");fflush(stdout);

	struct rte_mem_config *mem_config;
	void *rte_mem_cfg_addr;

	/* save the address primary process has mapped shared config to */
	rte_mem_cfg_addr = (void *) (uintptr_t) rte_config.mem_config->mem_cfg_addr;

    // printf("before munmap\n");fflush(stdout);
	/* unmap original config */
	munmap(rte_config.mem_config, sizeof(struct rte_mem_config));
    // printf("after munmap\n");fflush(stdout);

	/* remap the config at proper address */
	mem_config = (struct rte_mem_config *) mmap(rte_mem_cfg_addr,
			sizeof(*mem_config), PROT_READ | PROT_WRITE, MAP_SHARED,
			mem_cfg_fd, 0);
	if (mem_config == MAP_FAILED || mem_config != rte_mem_cfg_addr) {
		if (mem_config != MAP_FAILED)
			/* errno is stale, don't use */
			printf("Cannot mmap memory for rte_config at [%p], got [%p]"
				  " - please use '--base-virtaddr' option\n",
				  rte_mem_cfg_addr, mem_config);
		else
			printf("Cannot mmap memory for rte_config! error %i (%s)\n",
				  errno, strerror(errno));
	}
	close(mem_cfg_fd);

	rte_config.mem_config = mem_config;
}

/* Sets up rte_config structure with the pointer to shared memory config.*/
void rte_config_attach(void)
{
    rte_eal_config_attach();
    rte_eal_mcfg_wait_complete(rte_config.mem_config);
    rte_eal_config_reattach();
}


bool phys_addrs_available = true;

/* Return a pointer to the configuration structure */
struct rte_config *
rte_eal_get_configuration(void)
{
	return &rte_config;
}

/*
 * Get physical address of any mapped virtual address in the current process.
 */
phys_addr_t
rte_mem_virt2phy(const void *virtaddr)
{
	int fd, retval;
	uint64_t page, physaddr;
	unsigned long virt_pfn;
	int page_size;
	off_t offset;

	/* Cannot parse /proc/self/pagemap, no need to log errors everywhere */
	if (!phys_addrs_available)
		return RTE_BAD_PHYS_ADDR;

	/* standard page size */
	page_size = getpagesize();

	fd = open("/proc/self/pagemap", O_RDONLY);
	if (fd < 0) {
        printf("%s(): cannot open /proc/self/pagemap: %s\n",
        __func__, strerror(errno));
		return RTE_BAD_PHYS_ADDR;
	}

	virt_pfn = (unsigned long)virtaddr / page_size;
	offset = sizeof(uint64_t) * virt_pfn;
	if (lseek(fd, offset, SEEK_SET) == (off_t) -1) {
        printf("%s(): seek error in /proc/self/pagemap: %s\n",
            __func__, strerror(errno));
        close(fd);
		return RTE_BAD_PHYS_ADDR;
	}

	retval = read(fd, &page, PFN_MASK_SIZE);
	close(fd);
	if (retval < 0) {
        printf("%s(): cannot read /proc/self/pagemap: %s\n",
				__func__, strerror(errno));
		return RTE_BAD_PHYS_ADDR;
	} else if (retval != PFN_MASK_SIZE) {
        printf("%s(): read %d bytes from /proc/self/pagemap "
				"but expected %d:\n",
				__func__, retval, PFN_MASK_SIZE);
		return RTE_BAD_PHYS_ADDR;
	}

	/*
	 * the pfn (page frame number) are bits 0-54 (see
	 * pagemap.txt in linux Documentation)
	 */
	if ((page & 0x7fffffffffffffULL) == 0)
		return RTE_BAD_PHYS_ADDR;

	physaddr = ((page & 0x7fffffffffffffULL) * page_size)
		+ ((unsigned long)virtaddr % page_size);

	return physaddr;
}

static void
test_phys_addrs_available(void)
{
	uint64_t tmp;
	phys_addr_t physaddr;

	physaddr = rte_mem_virt2phy(&tmp);
	if (physaddr == RTE_BAD_PHYS_ADDR) {
        printf("Cannot obtain physical addresses: %s. "
        "Only vfio will function.\n",
        strerror(errno));

		phys_addrs_available = false;
	}
}

/*
 * This creates the memory mappings in the secondary process to match that of
 * the server process. It goes through each memory segment in the DPDK runtime
 * configuration and finds the hugepages which form that segment, mapping them
 * in order to form a contiguous block in the virtual memory space
 */
int rte_eal_hugepage_attach(void)
{
	const struct rte_mem_config *mcfg = rte_eal_get_configuration()->mem_config;
	struct hugepage_file *hp = NULL;
	unsigned num_hp = 0;
	unsigned i, s = 0; /* s used to track the segment number */
	unsigned max_seg = RTE_MAX_MEMSEG;
	off_t size = 0;
	int fd, fd_zero = -1, fd_hugepage = -1;

	// if (aslr_enabled() > 0) {
	// 	RTE_LOG(WARNING, EAL, "WARNING: Address Space Layout Randomization "
	// 			"(ASLR) is enabled in the kernel.\n");
	// 	RTE_LOG(WARNING, EAL, "   This may cause issues with mapping memory "
	// 			"into secondary processes\n");
	// }

	test_phys_addrs_available();


	fd_zero = open("/dev/zero", O_RDONLY);
	if (fd_zero < 0) {
        printf("Could not open /dev/zero\n");
        goto error;
	}
	fd_hugepage = open(eal_hugepage_info_path(), O_RDONLY);
	if (fd_hugepage < 0) {
        printf("Could not open %s\n", eal_hugepage_info_path());
        goto error;
	}

	/* map all segments into memory to make sure we get the addrs */
	for (s = 0; s < RTE_MAX_MEMSEG; ++s) {
		void *base_addr;

		/*
		 * the first memory segment with len==0 is the one that
		 * follows the last valid segment.
		 */
		if (mcfg->memseg[s].len == 0)
			break;

		/*
		 * fdzero is mmapped to get a contiguous block of virtual
		 * addresses of the appropriate memseg size.
		 * use mmap to get identical addresses as the primary process.
		 */
		base_addr = mmap(mcfg->memseg[s].addr, mcfg->memseg[s].len,
				 PROT_READ,
#ifdef RTE_ARCH_PPC_64
				 MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB,
#else
				 MAP_PRIVATE,
#endif
				 fd_zero, 0);
		if (base_addr == MAP_FAILED ||
		    base_addr != mcfg->memseg[s].addr) {
			max_seg = s;
			if (base_addr != MAP_FAILED) {
				/* errno is stale, don't use */
                printf("Could not mmap %llu bytes "
                "in /dev/zero at [%p], got [%p] - "
                "please use '--base-virtaddr' option\n",
                (unsigned long long)mcfg->memseg[s].len,
                mcfg->memseg[s].addr, base_addr);     
                munmap(base_addr, mcfg->memseg[s].len);
			} else {
				printf("Could not mmap %llu bytes "
					"in /dev/zero at [%p]: '%s'\n",
					(unsigned long long)mcfg->memseg[s].len,
					mcfg->memseg[s].addr, strerror(errno));
			}
			goto error;
		}
	}

	size = getFileSize(fd_hugepage);
	hp = mmap(NULL, size, PROT_READ, MAP_PRIVATE, fd_hugepage, 0);
	if (hp == MAP_FAILED) {
		printf("Could not mmap %s\n", eal_hugepage_info_path());
		goto error;
	}

	num_hp = size / sizeof(struct hugepage_file);
	printf("Analysing %u files\n", num_hp);

	s = 0;
	while (s < RTE_MAX_MEMSEG && mcfg->memseg[s].len > 0){
		void *addr, *base_addr;
		uintptr_t offset = 0;
		size_t mapping_size;
		/*
		 * free previously mapped memory so we can map the
		 * hugepages into the space
		 */
		base_addr = mcfg->memseg[s].addr;
		munmap(base_addr, mcfg->memseg[s].len);

		/* find the hugepages for this segment and map them
		 * we don't need to worry about order, as the server sorted the
		 * entries before it did the second mmap of them */
		for (i = 0; i < num_hp && offset < mcfg->memseg[s].len; i++){
			if (hp[i].memseg_id == (int)s){
				fd = open(hp[i].filepath, O_RDWR);
				if (fd < 0) {
					printf("Could not open %s\n",
						hp[i].filepath);
					goto error;
				}
				mapping_size = hp[i].size;
				addr = mmap(RTE_PTR_ADD(base_addr, offset),
						mapping_size, PROT_READ | PROT_WRITE,
						MAP_SHARED, fd, 0);
				close(fd); /* close file both on success and on failure */
				if (addr == MAP_FAILED ||
						addr != RTE_PTR_ADD(base_addr, offset)) {
					printf("Could not mmap %s\n",
						hp[i].filepath);
					goto error;
				}
				offset+=mapping_size;
			}
		}
		printf("Mapped segment %u of size 0x%llx\n", s,
				(unsigned long long)mcfg->memseg[s].len);
		s++;
	}
	/* unmap the hugepage config file, since we are done using it */
	munmap(hp, size);
	close(fd_zero);
	close(fd_hugepage);
	return 0;

error:
	for (i = 0; i < max_seg && mcfg->memseg[i].len > 0; i++)
		munmap(mcfg->memseg[i].addr, mcfg->memseg[i].len);
	if (hp != NULL && hp != MAP_FAILED)
		munmap(hp, size);
	if (fd_zero >= 0)
		close(fd_zero);
	if (fd_hugepage >= 0)
		close(fd_hugepage);
	return -1;
}

void mapping() {
    printf("Try to reconstruct the VA-PA mapping of the master dpdk process!\n");
    // this will open, mmap, and read mem_config struct from default_config_path. 
    // It also guarantees secondary process has the same VA-PA mapping for the mem_config struct. 
    rte_config_attach();
    rte_eal_hugepage_attach();
    printf("VA-PA mapping reconstruction succeeds!\n");
}

// int main(void) {
//     printf("Try to reconstruct the VA-PA mapping of the master dpdk process!\n");
//     // this will open, mmap, and read mem_config struct from default_config_path. 
//     // It also guarantees secondary process has the same VA-PA mapping for the mem_config struct. 
//     rte_config_attach();
//     rte_eal_hugepage_attach();
//     printf("VA-PA mapping reconstruction succeeds!\n");
// }