--
-- PostgreSQL database dump
--


-- Dumped from database version 16.13 (Ubuntu 16.13-0ubuntu0.24.04.1)
-- Dumped by pg_dump version 16.13 (Ubuntu 16.13-0ubuntu0.24.04.1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

ALTER TABLE IF EXISTS ONLY public.sense_prop DROP CONSTRAINT IF EXISTS sense_prop_sense_sense_id_foreign;
ALTER TABLE IF EXISTS ONLY public.sense_prop DROP CONSTRAINT IF EXISTS sense_prop_entry_seq_foreign;
ALTER TABLE IF EXISTS ONLY public.sense DROP CONSTRAINT IF EXISTS sense_entry_seq_foreign;
ALTER TABLE IF EXISTS ONLY public.restricted_readings DROP CONSTRAINT IF EXISTS restricted_readings_entry_seq_foreign;
ALTER TABLE IF EXISTS ONLY public.reading DROP CONSTRAINT IF EXISTS reading_kanji_kanji_id_foreign;
ALTER TABLE IF EXISTS ONLY public.okurigana DROP CONSTRAINT IF EXISTS okurigana_reading_reading_id_foreign;
ALTER TABLE IF EXISTS ONLY public.meaning DROP CONSTRAINT IF EXISTS meaning_kanji_kanji_id_foreign;
ALTER TABLE IF EXISTS ONLY public.kanji_text DROP CONSTRAINT IF EXISTS kanji_text_entry_seq_foreign;
ALTER TABLE IF EXISTS ONLY public.kana_text DROP CONSTRAINT IF EXISTS kana_text_entry_seq_foreign;
ALTER TABLE IF EXISTS ONLY public.gloss DROP CONSTRAINT IF EXISTS gloss_sense_sense_id_foreign;
ALTER TABLE IF EXISTS ONLY public.conjugation DROP CONSTRAINT IF EXISTS conjugation_entry_seq_foreign;
ALTER TABLE IF EXISTS ONLY public.conjugation DROP CONSTRAINT IF EXISTS conjugation_entry_from_foreign;
ALTER TABLE IF EXISTS ONLY public.conj_source_reading DROP CONSTRAINT IF EXISTS conj_source_reading_conjugation_conj_id_foreign;
ALTER TABLE IF EXISTS ONLY public.conj_prop DROP CONSTRAINT IF EXISTS conj_prop_conjugation_conj_id_foreign;
DROP INDEX IF EXISTS public.sense_seq_index;
DROP INDEX IF EXISTS public.sense_prop_tag_text_index;
DROP INDEX IF EXISTS public.sense_prop_seq_tag_text_index;
DROP INDEX IF EXISTS public.sense_prop_sense_id_tag_index;
DROP INDEX IF EXISTS public.restricted_readings_seq_reading_index;
DROP INDEX IF EXISTS public.reading_type_index;
DROP INDEX IF EXISTS public.reading_text_index;
DROP INDEX IF EXISTS public.reading_stat_common_index;
DROP INDEX IF EXISTS public.reading_kanji_id_index;
DROP INDEX IF EXISTS public.okurigana_reading_id_index;
DROP INDEX IF EXISTS public.meaning_text_index;
DROP INDEX IF EXISTS public.meaning_kanji_id_index;
DROP INDEX IF EXISTS public.kanji_text_text_index;
DROP INDEX IF EXISTS public.kanji_text_seq_index;
DROP INDEX IF EXISTS public.kanji_text_ord_index;
DROP INDEX IF EXISTS public.kanji_text_index;
DROP INDEX IF EXISTS public.kanji_text_common_index;
DROP INDEX IF EXISTS public.kanji_strokes_index;
DROP INDEX IF EXISTS public.kanji_stat_irregular_index;
DROP INDEX IF EXISTS public.kanji_stat_common_index;
DROP INDEX IF EXISTS public.kanji_radical_n_index;
DROP INDEX IF EXISTS public.kanji_radical_c_index;
DROP INDEX IF EXISTS public.kanji_grade_index;
DROP INDEX IF EXISTS public.kanji_freq_index;
DROP INDEX IF EXISTS public.kana_text_text_index;
DROP INDEX IF EXISTS public.kana_text_seq_index;
DROP INDEX IF EXISTS public.kana_text_ord_index;
DROP INDEX IF EXISTS public.kana_text_common_index;
DROP INDEX IF EXISTS public.gloss_sense_id_index;
DROP INDEX IF EXISTS public.conjugation_seq_index;
DROP INDEX IF EXISTS public.conjugation_from_index;
DROP INDEX IF EXISTS public.conj_source_reading_conj_id_text_index;
DROP INDEX IF EXISTS public.conj_prop_conj_id_index;
ALTER TABLE IF EXISTS ONLY public.sense_prop DROP CONSTRAINT IF EXISTS sense_prop_pkey;
ALTER TABLE IF EXISTS ONLY public.sense DROP CONSTRAINT IF EXISTS sense_pkey;
ALTER TABLE IF EXISTS ONLY public.restricted_readings DROP CONSTRAINT IF EXISTS restricted_readings_pkey;
ALTER TABLE IF EXISTS ONLY public.reading DROP CONSTRAINT IF EXISTS reading_pkey;
ALTER TABLE IF EXISTS ONLY public.okurigana DROP CONSTRAINT IF EXISTS okurigana_pkey;
ALTER TABLE IF EXISTS ONLY public.meaning DROP CONSTRAINT IF EXISTS meaning_pkey;
ALTER TABLE IF EXISTS ONLY public.kanji_text DROP CONSTRAINT IF EXISTS kanji_text_pkey;
ALTER TABLE IF EXISTS ONLY public.kanji DROP CONSTRAINT IF EXISTS kanji_pkey;
ALTER TABLE IF EXISTS ONLY public.kana_text DROP CONSTRAINT IF EXISTS kana_text_pkey;
ALTER TABLE IF EXISTS ONLY public.gloss DROP CONSTRAINT IF EXISTS gloss_pkey;
ALTER TABLE IF EXISTS ONLY public.entry DROP CONSTRAINT IF EXISTS entry_pkey;
ALTER TABLE IF EXISTS ONLY public.conjugation DROP CONSTRAINT IF EXISTS conjugation_pkey;
ALTER TABLE IF EXISTS ONLY public.conj_source_reading DROP CONSTRAINT IF EXISTS conj_source_reading_pkey;
ALTER TABLE IF EXISTS ONLY public.conj_prop DROP CONSTRAINT IF EXISTS conj_prop_pkey;
ALTER TABLE IF EXISTS public.sense_prop ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.sense ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.restricted_readings ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.reading ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.okurigana ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.meaning ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.kanji_text ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.kanji ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.kana_text ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.gloss ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.conjugation ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.conj_source_reading ALTER COLUMN id DROP DEFAULT;
ALTER TABLE IF EXISTS public.conj_prop ALTER COLUMN id DROP DEFAULT;
DROP SEQUENCE IF EXISTS public.sense_prop_id_seq;
DROP TABLE IF EXISTS public.sense_prop;
DROP SEQUENCE IF EXISTS public.sense_id_seq;
DROP TABLE IF EXISTS public.sense;
DROP SEQUENCE IF EXISTS public.restricted_readings_id_seq;
DROP TABLE IF EXISTS public.restricted_readings;
DROP SEQUENCE IF EXISTS public.reading_id_seq;
DROP TABLE IF EXISTS public.reading;
DROP SEQUENCE IF EXISTS public.okurigana_id_seq;
DROP TABLE IF EXISTS public.okurigana;
DROP SEQUENCE IF EXISTS public.meaning_id_seq;
DROP TABLE IF EXISTS public.meaning;
DROP SEQUENCE IF EXISTS public.kanji_text_id_seq;
DROP TABLE IF EXISTS public.kanji_text;
DROP SEQUENCE IF EXISTS public.kanji_id_seq;
DROP TABLE IF EXISTS public.kanji;
DROP SEQUENCE IF EXISTS public.kana_text_id_seq;
DROP TABLE IF EXISTS public.kana_text;
DROP SEQUENCE IF EXISTS public.gloss_id_seq;
DROP TABLE IF EXISTS public.gloss;
DROP TABLE IF EXISTS public.entry;
DROP SEQUENCE IF EXISTS public.conjugation_id_seq;
DROP TABLE IF EXISTS public.conjugation;
DROP SEQUENCE IF EXISTS public.conj_source_reading_id_seq;
DROP TABLE IF EXISTS public.conj_source_reading;
DROP SEQUENCE IF EXISTS public.conj_prop_id_seq;
DROP TABLE IF EXISTS public.conj_prop;
-- *not* dropping schema, since initdb creates it
--
-- Name: public; Type: SCHEMA; Schema: -; Owner: -
--

-- *not* creating schema, since initdb creates it


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: conj_prop; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.conj_prop (
    id integer NOT NULL,
    conj_id integer NOT NULL,
    conj_type integer NOT NULL,
    pos text NOT NULL,
    neg boolean,
    fml boolean
);


--
-- Name: conj_prop_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.conj_prop_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: conj_prop_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.conj_prop_id_seq OWNED BY public.conj_prop.id;


--
-- Name: conj_source_reading; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.conj_source_reading (
    id integer NOT NULL,
    conj_id integer NOT NULL,
    text text NOT NULL,
    source_text text NOT NULL
);


--
-- Name: conj_source_reading_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.conj_source_reading_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: conj_source_reading_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.conj_source_reading_id_seq OWNED BY public.conj_source_reading.id;


--
-- Name: conjugation; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.conjugation (
    id integer NOT NULL,
    seq integer NOT NULL,
    "from" integer NOT NULL,
    via integer
);


--
-- Name: conjugation_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.conjugation_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: conjugation_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.conjugation_id_seq OWNED BY public.conjugation.id;


--
-- Name: entry; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.entry (
    seq integer NOT NULL,
    content text NOT NULL,
    root_p boolean NOT NULL,
    n_kanji integer NOT NULL,
    n_kana integer NOT NULL,
    primary_nokanji boolean NOT NULL
);


--
-- Name: gloss; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gloss (
    id integer NOT NULL,
    sense_id integer NOT NULL,
    text text NOT NULL,
    ord integer NOT NULL
);


--
-- Name: gloss_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.gloss_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gloss_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gloss_id_seq OWNED BY public.gloss.id;


--
-- Name: kana_text; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kana_text (
    id integer NOT NULL,
    seq integer NOT NULL,
    text text NOT NULL,
    ord integer NOT NULL,
    common integer,
    common_tags text NOT NULL,
    conjugate_p boolean NOT NULL,
    nokanji boolean NOT NULL,
    best_kanji text
);


--
-- Name: kana_text_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kana_text_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kana_text_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kana_text_id_seq OWNED BY public.kana_text.id;


--
-- Name: kanji; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kanji (
    id integer NOT NULL,
    text text NOT NULL,
    radical_c integer NOT NULL,
    radical_n integer NOT NULL,
    grade integer,
    strokes integer NOT NULL,
    freq integer,
    stat_common integer NOT NULL,
    stat_irregular integer NOT NULL
);


--
-- Name: kanji_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kanji_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kanji_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kanji_id_seq OWNED BY public.kanji.id;


--
-- Name: kanji_text; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kanji_text (
    id integer NOT NULL,
    seq integer NOT NULL,
    text text NOT NULL,
    ord integer NOT NULL,
    common integer,
    common_tags text NOT NULL,
    conjugate_p boolean NOT NULL,
    nokanji boolean NOT NULL,
    best_kana text
);


--
-- Name: kanji_text_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kanji_text_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kanji_text_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kanji_text_id_seq OWNED BY public.kanji_text.id;


--
-- Name: meaning; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.meaning (
    id integer NOT NULL,
    kanji_id integer NOT NULL,
    text text NOT NULL
);


--
-- Name: meaning_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.meaning_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: meaning_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.meaning_id_seq OWNED BY public.meaning.id;


--
-- Name: okurigana; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.okurigana (
    id integer NOT NULL,
    reading_id integer NOT NULL,
    text text NOT NULL
);


--
-- Name: okurigana_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.okurigana_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: okurigana_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.okurigana_id_seq OWNED BY public.okurigana.id;


--
-- Name: reading; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reading (
    id integer NOT NULL,
    kanji_id integer NOT NULL,
    type text NOT NULL,
    text text NOT NULL,
    suffixp boolean NOT NULL,
    prefixp boolean NOT NULL,
    stat_common integer NOT NULL
);


--
-- Name: reading_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.reading_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: reading_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.reading_id_seq OWNED BY public.reading.id;


--
-- Name: restricted_readings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.restricted_readings (
    id integer NOT NULL,
    seq integer NOT NULL,
    reading text NOT NULL,
    text text NOT NULL
);


--
-- Name: restricted_readings_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.restricted_readings_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: restricted_readings_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.restricted_readings_id_seq OWNED BY public.restricted_readings.id;


--
-- Name: sense; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sense (
    id integer NOT NULL,
    seq integer NOT NULL,
    ord integer NOT NULL
);


--
-- Name: sense_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.sense_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: sense_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.sense_id_seq OWNED BY public.sense.id;


--
-- Name: sense_prop; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sense_prop (
    id integer NOT NULL,
    tag text NOT NULL,
    sense_id integer NOT NULL,
    text text NOT NULL,
    ord integer NOT NULL,
    seq integer NOT NULL
);


--
-- Name: sense_prop_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.sense_prop_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: sense_prop_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.sense_prop_id_seq OWNED BY public.sense_prop.id;


--
-- Name: conj_prop id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conj_prop ALTER COLUMN id SET DEFAULT nextval('public.conj_prop_id_seq'::regclass);


--
-- Name: conj_source_reading id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conj_source_reading ALTER COLUMN id SET DEFAULT nextval('public.conj_source_reading_id_seq'::regclass);


--
-- Name: conjugation id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conjugation ALTER COLUMN id SET DEFAULT nextval('public.conjugation_id_seq'::regclass);


--
-- Name: gloss id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gloss ALTER COLUMN id SET DEFAULT nextval('public.gloss_id_seq'::regclass);


--
-- Name: kana_text id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kana_text ALTER COLUMN id SET DEFAULT nextval('public.kana_text_id_seq'::regclass);


--
-- Name: kanji id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kanji ALTER COLUMN id SET DEFAULT nextval('public.kanji_id_seq'::regclass);


--
-- Name: kanji_text id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kanji_text ALTER COLUMN id SET DEFAULT nextval('public.kanji_text_id_seq'::regclass);


--
-- Name: meaning id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meaning ALTER COLUMN id SET DEFAULT nextval('public.meaning_id_seq'::regclass);


--
-- Name: okurigana id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.okurigana ALTER COLUMN id SET DEFAULT nextval('public.okurigana_id_seq'::regclass);


--
-- Name: reading id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reading ALTER COLUMN id SET DEFAULT nextval('public.reading_id_seq'::regclass);


--
-- Name: restricted_readings id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.restricted_readings ALTER COLUMN id SET DEFAULT nextval('public.restricted_readings_id_seq'::regclass);


--
-- Name: sense id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense ALTER COLUMN id SET DEFAULT nextval('public.sense_id_seq'::regclass);


--
-- Name: sense_prop id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense_prop ALTER COLUMN id SET DEFAULT nextval('public.sense_prop_id_seq'::regclass);


--
-- Name: conj_prop conj_prop_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conj_prop
    ADD CONSTRAINT conj_prop_pkey PRIMARY KEY (id);


--
-- Name: conj_source_reading conj_source_reading_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conj_source_reading
    ADD CONSTRAINT conj_source_reading_pkey PRIMARY KEY (id);


--
-- Name: conjugation conjugation_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conjugation
    ADD CONSTRAINT conjugation_pkey PRIMARY KEY (id);


--
-- Name: entry entry_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entry
    ADD CONSTRAINT entry_pkey PRIMARY KEY (seq);


--
-- Name: gloss gloss_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gloss
    ADD CONSTRAINT gloss_pkey PRIMARY KEY (id);


--
-- Name: kana_text kana_text_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kana_text
    ADD CONSTRAINT kana_text_pkey PRIMARY KEY (id);


--
-- Name: kanji kanji_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kanji
    ADD CONSTRAINT kanji_pkey PRIMARY KEY (id);


--
-- Name: kanji_text kanji_text_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kanji_text
    ADD CONSTRAINT kanji_text_pkey PRIMARY KEY (id);


--
-- Name: meaning meaning_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meaning
    ADD CONSTRAINT meaning_pkey PRIMARY KEY (id);


--
-- Name: okurigana okurigana_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.okurigana
    ADD CONSTRAINT okurigana_pkey PRIMARY KEY (id);


--
-- Name: reading reading_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reading
    ADD CONSTRAINT reading_pkey PRIMARY KEY (id);


--
-- Name: restricted_readings restricted_readings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.restricted_readings
    ADD CONSTRAINT restricted_readings_pkey PRIMARY KEY (id);


--
-- Name: sense sense_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense
    ADD CONSTRAINT sense_pkey PRIMARY KEY (id);


--
-- Name: sense_prop sense_prop_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense_prop
    ADD CONSTRAINT sense_prop_pkey PRIMARY KEY (id);


--
-- Name: conj_prop_conj_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX conj_prop_conj_id_index ON public.conj_prop USING btree (conj_id);


--
-- Name: conj_source_reading_conj_id_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX conj_source_reading_conj_id_text_index ON public.conj_source_reading USING btree (conj_id, text);


--
-- Name: conjugation_from_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX conjugation_from_index ON public.conjugation USING btree ("from");


--
-- Name: conjugation_seq_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX conjugation_seq_index ON public.conjugation USING btree (seq);


--
-- Name: gloss_sense_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX gloss_sense_id_index ON public.gloss USING btree (sense_id);


--
-- Name: kana_text_common_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kana_text_common_index ON public.kana_text USING btree (common);


--
-- Name: kana_text_ord_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kana_text_ord_index ON public.kana_text USING btree (ord);


--
-- Name: kana_text_seq_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kana_text_seq_index ON public.kana_text USING btree (seq);


--
-- Name: kana_text_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kana_text_text_index ON public.kana_text USING btree (text);


--
-- Name: kanji_freq_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_freq_index ON public.kanji USING btree (freq);


--
-- Name: kanji_grade_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_grade_index ON public.kanji USING btree (grade);


--
-- Name: kanji_radical_c_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_radical_c_index ON public.kanji USING btree (radical_c);


--
-- Name: kanji_radical_n_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_radical_n_index ON public.kanji USING btree (radical_n);


--
-- Name: kanji_stat_common_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_stat_common_index ON public.kanji USING btree (stat_common);


--
-- Name: kanji_stat_irregular_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_stat_irregular_index ON public.kanji USING btree (stat_irregular);


--
-- Name: kanji_strokes_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_strokes_index ON public.kanji USING btree (strokes);


--
-- Name: kanji_text_common_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_text_common_index ON public.kanji_text USING btree (common);


--
-- Name: kanji_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_text_index ON public.kanji USING btree (text);


--
-- Name: kanji_text_ord_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_text_ord_index ON public.kanji_text USING btree (ord);


--
-- Name: kanji_text_seq_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_text_seq_index ON public.kanji_text USING btree (seq);


--
-- Name: kanji_text_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX kanji_text_text_index ON public.kanji_text USING btree (text);


--
-- Name: meaning_kanji_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX meaning_kanji_id_index ON public.meaning USING btree (kanji_id);


--
-- Name: meaning_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX meaning_text_index ON public.meaning USING btree (text);


--
-- Name: okurigana_reading_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX okurigana_reading_id_index ON public.okurigana USING btree (reading_id);


--
-- Name: reading_kanji_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX reading_kanji_id_index ON public.reading USING btree (kanji_id);


--
-- Name: reading_stat_common_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX reading_stat_common_index ON public.reading USING btree (stat_common);


--
-- Name: reading_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX reading_text_index ON public.reading USING btree (text);


--
-- Name: reading_type_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX reading_type_index ON public.reading USING btree (type);


--
-- Name: restricted_readings_seq_reading_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX restricted_readings_seq_reading_index ON public.restricted_readings USING btree (seq, reading);


--
-- Name: sense_prop_sense_id_tag_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX sense_prop_sense_id_tag_index ON public.sense_prop USING btree (sense_id, tag);


--
-- Name: sense_prop_seq_tag_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX sense_prop_seq_tag_text_index ON public.sense_prop USING btree (seq, tag, text);


--
-- Name: sense_prop_tag_text_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX sense_prop_tag_text_index ON public.sense_prop USING btree (tag, text);


--
-- Name: sense_seq_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX sense_seq_index ON public.sense USING btree (seq);


--
-- Name: conj_prop conj_prop_conjugation_conj_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conj_prop
    ADD CONSTRAINT conj_prop_conjugation_conj_id_foreign FOREIGN KEY (conj_id) REFERENCES public.conjugation(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: conj_source_reading conj_source_reading_conjugation_conj_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conj_source_reading
    ADD CONSTRAINT conj_source_reading_conjugation_conj_id_foreign FOREIGN KEY (conj_id) REFERENCES public.conjugation(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: conjugation conjugation_entry_from_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conjugation
    ADD CONSTRAINT conjugation_entry_from_foreign FOREIGN KEY ("from") REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: conjugation conjugation_entry_seq_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.conjugation
    ADD CONSTRAINT conjugation_entry_seq_foreign FOREIGN KEY (seq) REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: gloss gloss_sense_sense_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gloss
    ADD CONSTRAINT gloss_sense_sense_id_foreign FOREIGN KEY (sense_id) REFERENCES public.sense(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: kana_text kana_text_entry_seq_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kana_text
    ADD CONSTRAINT kana_text_entry_seq_foreign FOREIGN KEY (seq) REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: kanji_text kanji_text_entry_seq_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kanji_text
    ADD CONSTRAINT kanji_text_entry_seq_foreign FOREIGN KEY (seq) REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: meaning meaning_kanji_kanji_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meaning
    ADD CONSTRAINT meaning_kanji_kanji_id_foreign FOREIGN KEY (kanji_id) REFERENCES public.kanji(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: okurigana okurigana_reading_reading_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.okurigana
    ADD CONSTRAINT okurigana_reading_reading_id_foreign FOREIGN KEY (reading_id) REFERENCES public.reading(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: reading reading_kanji_kanji_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reading
    ADD CONSTRAINT reading_kanji_kanji_id_foreign FOREIGN KEY (kanji_id) REFERENCES public.kanji(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: restricted_readings restricted_readings_entry_seq_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.restricted_readings
    ADD CONSTRAINT restricted_readings_entry_seq_foreign FOREIGN KEY (seq) REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: sense sense_entry_seq_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense
    ADD CONSTRAINT sense_entry_seq_foreign FOREIGN KEY (seq) REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: sense_prop sense_prop_entry_seq_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense_prop
    ADD CONSTRAINT sense_prop_entry_seq_foreign FOREIGN KEY (seq) REFERENCES public.entry(seq) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- Name: sense_prop sense_prop_sense_sense_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sense_prop
    ADD CONSTRAINT sense_prop_sense_sense_id_foreign FOREIGN KEY (sense_id) REFERENCES public.sense(id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--


