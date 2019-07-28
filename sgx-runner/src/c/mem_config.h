#ifndef __MEM_CONFIG_H
#define __MEM_CONFIG_H

#include <stdint.h>
#include <stdio.h>
#include <emmintrin.h>

#define RTE_MAGIC 19820526 /**< Magic number written by the main partition when ready. */

#define RTE_MAX_MEMSEG 256
#define RTE_MAX_TAILQ 32
#define RTE_MAX_NUMA_NODES 8
#define RTE_MAX_MEMZONE 2560
#define RTE_CACHE_LINE_SIZE 64

/**
 * Force a structure to be packed
 */
#define __rte_packed __attribute__((__packed__))

/**
 * The rte_rwlock_t type.
 *
 * cnt is -1 when write lock is held, and > 0 when read locks are held.
 */
typedef struct {
	volatile int32_t cnt; /**< -1 when W lock held, > 0 when R locks held. */
} rte_rwlock_t;


typedef uint64_t phys_addr_t; /**< Physical address definition. */

/** C extension macro for environments lacking C11 features. */
#if !defined(__STDC_VERSION__) || __STDC_VERSION__ < 201112L
#define RTE_STD_C11 __extension__
#else
#define RTE_STD_C11
#endif

/**
 * Physical memory segment descriptor.
 */
struct rte_memseg {
	phys_addr_t phys_addr;      /**< Start physical address. */
	RTE_STD_C11
	union {
		void *addr;         /**< Start virtual address. */
		uint64_t addr_64;   /**< Makes sure addr is always 64 bits */
	};
	size_t len;               /**< Length of the segment. */
	uint64_t hugepage_sz;       /**< The pagesize of underlying memory */
	int32_t socket_id;          /**< NUMA socket ID. */
	uint32_t nchannel;          /**< Number of channels. */
	uint32_t nrank;             /**< Number of ranks. */
#ifdef RTE_LIBRTE_XEN_DOM0
	 /**< store segment MFNs */
	uint64_t mfn[DOM0_NUM_MEMBLOCK];
#endif
} __rte_packed;

/**
 * A structure describing a memzone, which is a contiguous portion of
 * physical memory identified by a name.
 */
struct rte_memzone {

#define RTE_MEMZONE_NAMESIZE 32       /**< Maximum length of memory zone name.*/
	char name[RTE_MEMZONE_NAMESIZE];  /**< Name of the memory zone. */

	phys_addr_t phys_addr;            /**< Start physical address. */
	RTE_STD_C11
	union {
		void *addr;                   /**< Start virtual address. */
		uint64_t addr_64;             /**< Makes sure addr is always 64-bits */
	};
	size_t len;                       /**< Length of the memzone. */

	uint64_t hugepage_sz;             /**< The page size of underlying memory */

	int32_t socket_id;                /**< NUMA socket ID. */

	uint32_t flags;                   /**< Characteristics of this memzone. */
	uint32_t memseg_id;               /**< Memseg it belongs. */
} __attribute__((__packed__));
/*
 * Tail queue definitions.
 */
#define	_TAILQ_HEAD(name, type, qual)					\
struct name {								\
	qual type *tqh_first;		/* first element */		\
	qual type *qual *tqh_last;	/* addr of last next element */	\
}   

#define TAILQ_HEAD(name, type)	_TAILQ_HEAD(name, struct type,)
TAILQ_HEAD(rte_tailq_entry_head, rte_tailq_entry);

#define RTE_TAILQ_NAMESIZE 32
/**
 * The structure defining a tailq header entry for storing
 * in the rte_config structure in shared memory. Each tailq
 * is identified by name.
 * Any library storing a set of objects e.g. rings, mempools, hash-tables,
 * is recommended to use an entry here, so as to make it easy for
 * a multi-process app to find already-created elements in shared memory.
 */
struct rte_tailq_head {
	struct rte_tailq_entry_head tailq_head; /**< NOTE: must be first element */
	char name[RTE_TAILQ_NAMESIZE];
};

/**
 * The rte_spinlock_t type.
 */
typedef struct {
	volatile int locked; /**< lock status 0 = unlocked, 1 = locked */
} rte_spinlock_t;

/*
 * List definitions.
 */
#define	LIST_HEAD(name, type)						\
struct name {								\
	struct type *lh_first;	/* first element */			\
}

#define	LIST_ENTRY(type)						\
struct {								\
	struct type *le_next;	/* next element */			\
	struct type **le_prev;	/* address of previous next element */	\
}

enum elem_state {
	ELEM_FREE = 0,
	ELEM_BUSY,
	ELEM_PAD  /* element is a padding-only header */
};

/**
 * Force alignment
 */
#define __rte_aligned(a) __attribute__((__aligned__(a)))

/**
 * Force alignment to cache line.
 */
#define __rte_cache_aligned __rte_aligned(RTE_CACHE_LINE_SIZE)


struct malloc_elem {
	struct malloc_heap *heap;
	struct malloc_elem *volatile prev;      /* points to prev elem in memseg */
	LIST_ENTRY(malloc_elem) free_list;      /* list of free elements in heap */
	const struct rte_memseg *ms;
	volatile enum elem_state state;
	uint32_t pad;
	size_t size;
#ifdef RTE_MALLOC_DEBUG
	uint64_t header_cookie;         /* Cookie marking start of data */
	                                /* trailer cookie at start + size */
#endif
} __rte_cache_aligned;

/* Number of free lists per heap, grouped by size. */
#define RTE_HEAP_NUM_FREELISTS  13

/**
 * Structure to hold malloc heap
 */
struct malloc_heap {
	rte_spinlock_t lock;
	LIST_HEAD(, malloc_elem) free_head[RTE_HEAP_NUM_FREELISTS];
	unsigned alloc_count;
	size_t total_size;
} __rte_cache_aligned;

/**
 * the structure for the memory configuration for the RTE.
 * Used by the rte_config structure. It is separated out, as for multi-process
 * support, the memory details should be shared across instances
 */
struct rte_mem_config {
	volatile uint32_t magic;   /**< Magic number - Sanity check. */

	/* memory topology */
	uint32_t nchannel;    /**< Number of channels (0 if unknown). */
	uint32_t nrank;       /**< Number of ranks (0 if unknown). */

	/**
	 * current lock nest order
	 *  - qlock->mlock (ring/hash/lpm)
	 *  - mplock->qlock->mlock (mempool)
	 * Notice:
	 *  *ALWAYS* obtain qlock first if having to obtain both qlock and mlock
	 */
	rte_rwlock_t mlock;   /**< only used by memzone LIB for thread-safe. */
	rte_rwlock_t qlock;   /**< used for tailq operation for thread safe. */
	rte_rwlock_t mplock;  /**< only used by mempool LIB for thread-safe. */

	uint32_t memzone_cnt; /**< Number of allocated memzones */

	/* memory segments and zones */
	struct rte_memseg memseg[RTE_MAX_MEMSEG];    /**< Physmem descriptors. */
	struct rte_memzone memzone[RTE_MAX_MEMZONE]; /**< Memzone descriptors. */

	struct rte_tailq_head tailq_head[RTE_MAX_TAILQ]; /**< Tailqs for objects */

	/* Heaps of Malloc per socket */
	struct malloc_heap malloc_heaps[RTE_MAX_NUMA_NODES];

	/* address of mem_config in primary process. used to map shared config into
	 * exact same address the primary process maps it.
	 */
	uint64_t mem_cfg_addr;
} __attribute__((__packed__));

#define RTE_MAX_LCORE 128

/**
 * The type of process in a linuxapp, multi-process setup
 */
enum rte_proc_type_t {
	RTE_PROC_AUTO = -1,   /* allow auto-detection of primary/secondary */
	RTE_PROC_PRIMARY = 0, /* set to zero, so primary is the default */
	RTE_PROC_SECONDARY,

	RTE_PROC_INVALID
};

/**
 * The lcore role (used in RTE or not).
 */
enum rte_lcore_role_t {
	ROLE_RTE,
	ROLE_OFF,
	ROLE_SERVICE,
};

/**
 * The global RTE configuration structure.
 */
struct rte_config {
	uint32_t master_lcore;       /**< Id of the master lcore */
	uint32_t lcore_count;        /**< Number of available logical cores. */
	uint32_t service_lcore_count;/**< Number of available service cores. */
	enum rte_lcore_role_t lcore_role[RTE_MAX_LCORE]; /**< State of cores. */

	/** Primary or secondary configuration */
	enum rte_proc_type_t process_type;

	/**
	 * Pointer to memory configuration, which may be shared across multiple
	 * DPDK instances
	 */
	struct rte_mem_config *mem_config;
} __attribute__((__packed__));

void rte_pause(void)
{
	_mm_pause();
}

const char *default_mem_config_path = "/var/run/.netbricks-outside_config";

const char *eal_runtime_config_path(void)
{
    return default_mem_config_path;
}


#endif // __MEM_CONFIG_H