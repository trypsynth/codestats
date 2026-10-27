// expect: lang="C" total=16 code=10 comment=2 blank=4 shebang=0
/* Simple linked list interface. */
#ifndef LIST_H
#define LIST_H

#include <stddef.h>

typedef struct node {
	int value;
	struct node *next;
} node;

node *list_push(node *head, int value);
size_t list_len(const node *head);

#endif
