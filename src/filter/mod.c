#include <libavfilter/avfilter.h>

typedef struct FilterGraph {
    AVFilterGraph* graph;
} FilterGraph;

FilterGraph* ffw_filter_graph_init();
int ffw_filter_graph_config(FilterGraph* fg);
void ffw_filter_graph_free(FilterGraph* fg);

FilterGraph* ffw_filter_graph_init() {
    AVFilterGraph* graph = avfilter_graph_alloc();
    if (graph == NULL) {
        return NULL;
    }

    FilterGraph* res = malloc(sizeof(FilterGraph));
    if (res == NULL) {
        return NULL;
    }

    res->graph = graph;

    return res;
}

int ffw_filter_graph_config(FilterGraph* fg) {
    return avfilter_graph_config(fg->graph, NULL);
}

void ffw_filter_graph_free(FilterGraph* fg) {
    if (fg == NULL) {
        return;
    }

    avfilter_graph_free(&fg->graph);
    free(fg);
}

typedef struct Filter {
    AVFilterContext* context;
    AVDictionary* options;
} Filter;

Filter* ffw_filter_alloc(FilterGraph* fg, const char* name);
int ffw_filter_init(Filter* filter);
int ffw_filter_set_initial_option(Filter* filter, const char* key, const char* value);
int ffw_filter_link(Filter* filter_a, unsigned int output, Filter* filter_b, unsigned int input);
void ffw_filter_free(Filter* name);

Filter* ffw_filter_alloc(FilterGraph* fg, const char* name) {
    const AVFilter* filter = avfilter_get_by_name(name);
    if (filter == NULL) {
        return NULL;
    }

    AVFilterContext* context = avfilter_graph_alloc_filter(fg->graph, filter, NULL);
    if (context == NULL) {
        return NULL;
    }

    Filter* res = malloc(sizeof(Filter));
    if (res == NULL) {
        return NULL;
    }

    res->context = context;
    res->options = NULL;

    return res;
}

int ffw_filter_init(Filter* filter) {
    int ret = avfilter_init_dict(filter->context, &filter->options);
    
    av_dict_free(&filter->options);
    
    filter->options = NULL;

    return ret;
}

int ffw_filter_set_initial_option(Filter* filter, const char* key, const char* value) {
    return av_dict_set(&filter->options, key, value, 0);
}

int ffw_filter_link(Filter* filter_a, unsigned int output, Filter* filter_b, unsigned int input) {
    return avfilter_link(filter_a->context, output, filter_b->context, input);
}

void ffw_filter_free(Filter* filter) {
    if (filter == NULL) {
        return;
    }

    avfilter_free(filter->context);
    if (filter->options != NULL) {
        av_dict_free(&filter->options);
    }
    free(filter);
}
