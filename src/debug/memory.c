#include <debug/debug.h>
#include <libc/ctype.h>

static void print_hexa(const char *ptr, const size_t bytes)
{
	size_t n;

	for(n = 0; n < bytes; ++n)
		printf("%x ", ((int) ptr[n]) & 0xff); // TODO Align
}

static void print_chars(const char *ptr, const size_t bytes)
{
	size_t n;

	printf(" |");
	for(n = 0; n < bytes; ++n)
		printf("%c", (isprint(ptr[n]) ? ptr[n] : '.'));
	printf("|\n");
}

void print_memory(const char *src, const size_t n)
{
	size_t i, count;

	for(i = 0; i < n; i += 16)
	{
		count = MIN(n - i, 16);
		printf("%p ", src + i); // TODO Align
		print_hexa(src + i, count);
		print_chars(src + i, count);
	}
}
