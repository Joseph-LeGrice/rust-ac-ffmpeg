#include <libavfilter/avfilter.h>
#include <libavfilter/buffersrc.h>
#include <libavfilter/buffersink.h>

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
    AVFrame* sinkframe;
} Filter;

Filter* ffw_filter_alloc(FilterGraph* fg, const char* name);
int ffw_filter_init(Filter* filter);
int ffw_filter_set_initial_option(Filter* filter, const char* key, const char* value);
int ffw_filter_link(Filter* filter_a, unsigned int output, Filter* filter_b, unsigned int input);
int ffw_filter_push_frame(Filter* buffersrc, AVFrame* frame);
int ffw_filter_take_frame(Filter* buffersink, AVFrame** frame);
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
    res->sinkframe = NULL;

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

int ffw_filter_push_frame(Filter* buffersrc, AVFrame* frame) {
    return av_buffersrc_add_frame(buffersrc->context, frame);
}

int ffw_filter_take_frame(Filter* buffersink, AVFrame** frame) {
    if (buffersink->sinkframe == NULL) {
        buffersink->sinkframe = av_frame_alloc();
    }

    int ret = av_buffersink_get_frame(buffersink->context, buffersink->sinkframe);

    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
        return 0;
    } else if (ret < 0) {
        return ret;
    }

    *frame = av_frame_clone(buffersink->sinkframe);

    return 1;
}

void ffw_filter_free(Filter* filter) {
    if (filter == NULL) {
        return;
    }

    avfilter_free(filter->context);
    if (filter->options != NULL) {
        av_dict_free(&filter->options);
    }
    if (filter->sinkframe != NULL) {
        av_frame_free(&filter->sinkframe);
    }
    free(filter);
}
