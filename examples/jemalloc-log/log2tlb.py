import sys

log_name = sys.argv[1]
print log_name

f = open(log_name, "r")
tlb_count = 0
for line in f:
    line = int(line.split()[1])
    print line
    if line <= 4096:
        tlb_count += 1
    elif line <= 2097152:
        tlb_count += 1



